//! Path-resolution and sensitive-path guards used by every fs / search tool.
//!
//! Lives in its own submodule because it is pure, has no AI / LLM coupling,
//! and is the natural first slice of the long-term `ai_tools/` split (see
//! `kaku-gui/AGENTS.md`). Keeping it isolated also makes the security check
//! easy to audit independently of the dispatcher in `mod.rs`.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolve the user's home directory from the environment.
///
/// Single source of truth for `$HOME` lookups in this crate so the failure
/// mode (panic / silent-empty / Result) does not drift between call sites.
pub(crate) fn home() -> Result<PathBuf> {
    let h = std::env::var_os("HOME").context("HOME not set")?;
    Ok(PathBuf::from(h))
}

/// Expand a leading `~`, `~/`, `$HOME`, `$HOME/`, `${HOME}`, or `${HOME}/`
/// in `s` to the user's home directory. Returns `None` when `s` carries
/// none of those forms (caller decides the fallback) or when `$HOME` is
/// unset.
pub(crate) fn expand_user_prefix(s: &str) -> Option<PathBuf> {
    let home = home().ok()?;
    if s == "~" || s == "$HOME" || s == "${HOME}" {
        return Some(home);
    }
    for prefix in ["~/", "$HOME/", "${HOME}/"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return Some(home.join(rest));
        }
    }
    None
}

/// Refuse reads of well-known credential / system-secret locations, even when
/// the caller passes an absolute or `~/`-prefixed path (both of which bypass
/// the cwd sandbox). Best-effort canonicalization: on ENOENT we compare the
/// raw path so a file about to be created in a blocked directory is still
/// caught.
pub(crate) fn reject_if_sensitive(path: &Path) -> Result<()> {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut blocked: Vec<PathBuf> = vec![
        PathBuf::from("/etc/shadow"),
        PathBuf::from("/etc/sudoers"),
        PathBuf::from("/etc/sudoers.d"),
        PathBuf::from("/private/etc/shadow"),
        PathBuf::from("/private/etc/sudoers"),
        PathBuf::from("/private/etc/sudoers.d"),
    ];
    if let Ok(home) = home() {
        for rel in [
            ".ssh",
            ".aws/credentials",
            ".gnupg",
            ".config/kaku/assistant.toml",
            ".config/kaku/secrets",
        ] {
            blocked.push(home.join(rel));
        }
    }
    for b in &blocked {
        let b_canon = std::fs::canonicalize(b).unwrap_or_else(|_| b.clone());
        if canon == b_canon || canon.starts_with(&b_canon) {
            anyhow::bail!(
                "refused: '{}' is a protected secret location",
                path.display()
            );
        }
    }

    // Name-pattern guard: credential files live anywhere, not just in the
    // fixed locations above. Check both the requested name and the
    // canonicalized name so a symlink cannot launder a `.env` past the guard.
    for candidate in [path, canon.as_path()] {
        if let Some(name) = candidate.file_name().and_then(|n| n.to_str()) {
            if is_sensitive_filename(name) {
                anyhow::bail!("refused: '{}' looks like a credential file", path.display());
            }
        }
    }
    Ok(())
}

/// File-name patterns that commonly hold secrets regardless of directory
/// (`.env`, private keys, PEM material). Obvious example/template files are
/// allowed because they are committed to repos on purpose and carry no
/// secrets.
fn is_sensitive_filename(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    // `.env.example`, `config.pem.sample`, `secrets.key.template`, ... are
    // placeholders, not real credentials.
    const ALLOW_SUFFIXES: [&str; 4] = [".example", ".sample", ".template", ".dist"];
    if ALLOW_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
        return false;
    }

    // Dotenv files: `.env`, `.env.local`, `.env.production`, ...
    if lower == ".env" || lower.starts_with(".env.") {
        return true;
    }

    // Private key material by extension.
    if lower.ends_with(".pem") || lower.ends_with(".key") {
        return true;
    }

    // Well-known SSH private key file names. Public `.pub` siblings do not
    // match the exact names, so they stay readable.
    matches!(
        lower.as_str(),
        "id_rsa" | "id_dsa" | "id_ecdsa" | "id_ed25519"
    )
}

/// Handles `~/…` expansion and relative paths (resolved against `cwd`).
///
/// Tool paths only accept `~` / `~/`; `$HOME` is not expanded here because
/// AI models do not pass shell-variable references through this path,
/// and silently expanding them would change tool-call semantics.
pub(crate) fn resolve(path: &str, cwd: &str) -> Result<PathBuf> {
    if path == "~" || path.starts_with("~/") {
        let home = home()?;
        return Ok(if path == "~" {
            home
        } else {
            home.join(&path[2..])
        });
    }
    if path.starts_with('/') {
        Ok(PathBuf::from(path))
    } else {
        Ok(PathBuf::from(cwd).join(path))
    }
}

/// Relative tool paths must stay inside the current project. Absolute and
/// `~/` paths remain explicit opt-ins, but `../../…` should not quietly mutate
/// files outside the pane's cwd while the approval prompt shows a relative path.
pub(crate) fn reject_relative_cwd_escape(raw_path: &str, resolved: &Path, cwd: &str) -> Result<()> {
    if raw_path.starts_with('/') || raw_path.starts_with("~/") || raw_path == "~" {
        return Ok(());
    }

    let canon_cwd =
        std::fs::canonicalize(cwd).with_context(|| format!("resolve working directory '{cwd}'"))?;
    if let Ok(canon_path) = std::fs::canonicalize(resolved) {
        if !canon_path.starts_with(&canon_cwd) {
            anyhow::bail!(
                "path '{}' resolves outside the working directory; \
                 use an absolute path to access it",
                raw_path
            );
        }
        return Ok(());
    }

    let mut existing = resolved.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            break;
        }
    }
    if existing.exists() {
        let canon_existing = std::fs::canonicalize(&existing)
            .with_context(|| format!("resolve '{}'", existing.display()))?;
        if !canon_existing.starts_with(&canon_cwd) {
            anyhow::bail!(
                "path '{}' resolves outside the working directory; \
                 use an absolute path to access it",
                raw_path
            );
        }
    }

    let mut lexical = canon_cwd.clone();
    for component in Path::new(raw_path).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => lexical.push(part),
            std::path::Component::ParentDir => {
                lexical.pop();
                if !lexical.starts_with(&canon_cwd) {
                    anyhow::bail!(
                        "path '{}' resolves outside the working directory; \
                         use an absolute path to access it",
                        raw_path
                    );
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_expands_tilde() {
        let home = std::env::var("HOME").expect("HOME not set");
        assert_eq!(
            resolve("~/foo", "/tmp").unwrap(),
            PathBuf::from(&home).join("foo")
        );
        assert_eq!(resolve("~", "/tmp").unwrap(), PathBuf::from(&home));
    }

    #[test]
    fn resolve_absolute_unchanged() {
        assert_eq!(
            resolve("/etc/passwd", "/tmp").unwrap(),
            PathBuf::from("/etc/passwd")
        );
    }

    #[test]
    fn resolve_relative_to_cwd() {
        assert_eq!(
            resolve("src/main.rs", "/project").unwrap(),
            PathBuf::from("/project/src/main.rs")
        );
    }

    #[test]
    fn reject_if_sensitive_blocks_ssh() {
        let home = std::env::var("HOME").expect("HOME not set");
        let ssh = PathBuf::from(&home).join(".ssh");
        let err = reject_if_sensitive(&ssh).expect_err("must reject ~/.ssh");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn reject_if_sensitive_blocks_assistant_config() {
        let home = std::env::var("HOME").expect("HOME not set");
        let assistant_config = PathBuf::from(&home).join(".config/kaku/assistant.toml");
        let err = reject_if_sensitive(&assistant_config).expect_err("must reject assistant config");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn reject_if_sensitive_allows_normal_paths() {
        // /tmp is not in the blocked list; resolve_if_sensitive must Ok it.
        assert!(reject_if_sensitive(&PathBuf::from("/tmp")).is_ok());
    }

    #[test]
    fn relative_cwd_escape_rejects_parent_traversal_outside_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("project");
        std::fs::create_dir(&cwd).unwrap();
        let raw = "../outside.txt";
        let resolved = resolve(raw, cwd.to_str().unwrap()).unwrap();
        let err = reject_relative_cwd_escape(raw, &resolved, cwd.to_str().unwrap())
            .expect_err("must reject cwd escape");
        assert!(err.to_string().contains("outside the working directory"));
    }

    #[test]
    fn relative_cwd_escape_allows_nested_missing_paths() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("project");
        std::fs::create_dir(&cwd).unwrap();
        let raw = "src/generated/file.txt";
        let resolved = resolve(raw, cwd.to_str().unwrap()).unwrap();
        reject_relative_cwd_escape(raw, &resolved, cwd.to_str().unwrap()).unwrap();
    }

    // ─── Sandbox audit additions ─────────────────────────────────────────
    // The original tests cover the common case (~/.ssh, assistant.toml,
    // single-level cwd escape). The cases below tighten coverage on attacks
    // that look subtle: symlink escape, deep parent traversal, multiple
    // hard-coded sensitive locations not previously exercised.

    #[test]
    fn reject_if_sensitive_blocks_aws_credentials() {
        let home = std::env::var("HOME").expect("HOME not set");
        let aws = PathBuf::from(&home).join(".aws/credentials");
        let err = reject_if_sensitive(&aws).expect_err("must reject ~/.aws/credentials");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn reject_if_sensitive_blocks_gnupg() {
        let home = std::env::var("HOME").expect("HOME not set");
        let gpg = PathBuf::from(&home).join(".gnupg");
        let err = reject_if_sensitive(&gpg).expect_err("must reject ~/.gnupg");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn reject_if_sensitive_blocks_descendant_of_sensitive_dir() {
        // ~/.ssh/id_rsa, ~/.ssh/config, etc must all be blocked because
        // ~/.ssh as a directory is in the blocklist (starts_with semantic).
        let home = std::env::var("HOME").expect("HOME not set");
        let key = PathBuf::from(&home).join(".ssh/id_rsa");
        let err = reject_if_sensitive(&key).expect_err("must reject ~/.ssh/id_rsa");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn reject_if_sensitive_blocks_etc_shadow_via_private_alias() {
        // macOS exposes /etc/shadow at both /etc/shadow and /private/etc/shadow.
        // The blocklist includes both. Verify the /private variant explicitly.
        let path = PathBuf::from("/private/etc/sudoers");
        let err = reject_if_sensitive(&path).expect_err("must reject /private/etc/sudoers");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    #[cfg(unix)]
    fn reject_if_sensitive_follows_symlink_to_blocked_dir() {
        // Attack shape: a symlink inside the project points to ~/.ssh. If
        // canonicalize() resolves the symlink before comparing, the blocklist
        // catches it. If it didn't, exfiltration via fs_read would bypass the
        // guard. This is exactly what canonicalize() is for.
        use std::os::unix::fs as unix_fs;
        let dir = tempfile::tempdir().unwrap();
        let home = std::env::var("HOME").expect("HOME not set");
        let target = PathBuf::from(&home).join(".ssh");
        if !target.exists() {
            // ~/.ssh might not exist on this machine; skip the realism check
            // and just assert the lexical match still triggers (covered by
            // reject_if_sensitive_blocks_ssh).
            return;
        }
        let link = dir.path().join("ssh_link");
        unix_fs::symlink(&target, &link).unwrap();
        let err = reject_if_sensitive(&link)
            .expect_err("symlink to ~/.ssh must be rejected via canonicalize");
        assert!(err.to_string().contains("protected secret location"));
    }

    #[test]
    fn relative_cwd_escape_rejects_deep_parent_traversal() {
        // ../../../../etc/passwd is the textbook directory-traversal payload.
        // The lexical fallback must catch it even when none of the intermediate
        // parents exist on disk.
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("project");
        std::fs::create_dir(&cwd).unwrap();
        let raw = "../../../../etc/passwd";
        let resolved = resolve(raw, cwd.to_str().unwrap()).unwrap();
        let err = reject_relative_cwd_escape(raw, &resolved, cwd.to_str().unwrap())
            .expect_err("deep ../ chain must be rejected");
        assert!(err.to_string().contains("outside the working directory"));
    }

    #[test]
    fn relative_cwd_escape_rejects_mixed_traversal() {
        // ./foo/../../escape is a sneakier variant: it climbs out of cwd via
        // a relative path that visually looks innocent (./foo/...). The
        // lexical walk must catch the net upward movement.
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("project");
        std::fs::create_dir(&cwd).unwrap();
        let raw = "./foo/../../escape.txt";
        let resolved = resolve(raw, cwd.to_str().unwrap()).unwrap();
        let err = reject_relative_cwd_escape(raw, &resolved, cwd.to_str().unwrap())
            .expect_err("mixed ./ + ../ traversal must be rejected");
        assert!(err.to_string().contains("outside the working directory"));
    }

    #[test]
    fn relative_cwd_escape_allows_within_cwd_traversal() {
        // foo/../bar.txt is just bar.txt, no escape. Must not false-positive.
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("project");
        std::fs::create_dir(&cwd).unwrap();
        let raw = "foo/../bar.txt";
        let resolved = resolve(raw, cwd.to_str().unwrap()).unwrap();
        reject_relative_cwd_escape(raw, &resolved, cwd.to_str().unwrap())
            .expect("in-cwd traversal must be allowed");
    }

    // ─── Credential file-name guard ───────────────────────────────────
    //
    // `reject_if_sensitive` blocks well-known credential file names
    // (`.env`, `*.pem`, `*.key`, SSH private keys) in any directory, while
    // still allowing committed example/template files.
    #[test]
    fn reject_if_sensitive_blocks_project_env_file() {
        let dir = tempfile::tempdir().unwrap();
        let env_file = dir.path().join(".env");
        std::fs::write(&env_file, "API_KEY=secret").unwrap();
        assert!(
            reject_if_sensitive(&env_file).is_err(),
            "a project-local .env must be refused"
        );
    }

    #[test]
    fn reject_if_sensitive_blocks_dotenv_variants_and_keys() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            ".env.local",
            ".env.production",
            "server.pem",
            "tls.key",
            "id_rsa",
            "id_ed25519",
        ] {
            let f = dir.path().join(name);
            std::fs::write(&f, "secret").unwrap();
            assert!(
                reject_if_sensitive(&f).is_err(),
                "{} should be refused as a credential file",
                name
            );
        }
    }

    #[test]
    fn reject_if_sensitive_allows_env_examples_and_public_keys() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            ".env.example",
            ".env.sample",
            "config.pem.template",
            "id_rsa.pub",
            "README.md",
        ] {
            let f = dir.path().join(name);
            std::fs::write(&f, "not a secret").unwrap();
            assert!(
                reject_if_sensitive(&f).is_ok(),
                "{} should remain readable",
                name
            );
        }
    }
}
