# Kaku Agent Guide

## Project

Kaku is a macOS-native terminal emulator forked from WezTerm and optimized for AI coding workflows.

## Repository Map

- `kaku/` - CLI entry point and command flows.
- `kaku-gui/` - GUI, rendering, window lifecycle, input, mouse handling, AI chat, and the `k` helper binary.
- `mux/` - tabs, panes, domains, and client/server state.
- `term/` - terminal emulation and screen state.
- `termwiz/` - terminal UI primitives.
- `config/` - Lua config loading, schema behavior, proxy settings, and versioned defaults.
- `window/` - platform windowing layer.
- `lua-api-crates/` - Rust-to-Lua API bindings.
- `crates/` - shared utility crates, including Kaku-specific AI helpers.
- `assets/` - app resources, bundled config, shell integration, and vendor assets.
- `scripts/` - build and release helpers.
- `docs/` - user and developer documentation, including CLI, configuration, and feature docs.
- `.github/workflows/ci.yml` - primary CI workflow.
- `.github/RELEASE_NOTES.md` - source for GitHub Release title and release copy.

## Commands

```bash
make fmt
make fmt-check
make check
make test
make dev
make app
./scripts/build.sh
./scripts/check_config_release_readiness.sh
./scripts/check_release_config.sh
./scripts/check_release_notes.sh
```

`make fmt` requires the nightly Rust toolchain. `make app` builds the app bundle path used by GUI, rendering, and AI overlay verification.

## Working Rules

- Work on the current branch unless the maintainer asks otherwise.
- Keep changes local to one crate or subsystem when possible.
- Prefer targeted search over repository-wide scans.
- Inspect public APIs and cross-crate boundaries before changing shared behavior.
- For issue and PR replies, draft concise text and get maintainer approval before posting.
- Do not modify files outside this repository without showing the intended change and getting explicit confirmation.
- Do not add instructions for the removed `website/` tree unless that directory exists in the current worktree.

## Investigation Order

When scope is incomplete, inspect in this order:

1. User-provided repro, failing command, or failing test.
2. Entry point for the behavior, usually `kaku/src/main.rs`, `kaku/src/cli/`, or `kaku-gui/src/main.rs`.
3. Owning subsystem document and target crate.
4. Immediate cross-crate boundary used by the call path.
5. Narrow tests, fixtures, snapshots, or scripts that reproduce the behavior.

For AI-facing behavior, inspect in this order:

1. CLI or assistant configuration under `kaku/src/ai_config/`, `kaku/src/assistant_config.rs`, and `config/src/proxy.rs`.
2. GUI AI state and transport under `kaku-gui/src/ai_*`, `kaku-gui/src/ai_chat_engine/`, and `kaku-gui/src/cli_chat/`.
3. Overlay UI under `kaku-gui/src/overlay/ai_chat/`.
4. Shared helpers in `crates/kaku-ai-utils/`.

## Subsystem Guides

| Subsystem | Guide | Scope |
|---|---|---|
| GUI | `kaku-gui/AGENTS.md` | Rendering, window lifecycle, input, mouse |
| Mux | `mux/AGENTS.md` | Tabs, panes, domains, client/server |
| Terminal | `term/AGENTS.md` | VT emulation, screen buffer, PTY-facing behavior |
| Config | `config/AGENTS.md` | Lua loading, schema, config reload |
| Termwiz | `termwiz/AGENTS.md` | TUI primitives and widgets |
| Lua API | `lua-api-crates/AGENTS.md` | Rust-to-Lua bindings |
| Crates | `crates/AGENTS.md` | Shared utility crates |

## Verification

| Change type | Command |
|---|---|
| Rust compile check | `make check` |
| Rust logic change | `make test` |
| Formatting | `make fmt-check` |
| GUI or rendering change | `make app` |
| Config release change | `./scripts/check_config_release_readiness.sh` and `./scripts/check_release_config.sh` |
| Release note change | `./scripts/check_release_notes.sh` |
| Release-adjacent change | `make fmt && make check && make test`, then `make app` |

For GUI or rendering issues, read `kaku-gui/AGENTS.md` first and verify with `make app`, not only `make dev`.

## Current Risk Areas

- AI chat and shell flows are active product surfaces. Preserve `fast_model`, proxy config, inline `#` query status, syntax highlighting, approval flow, and conversation state behavior.
- Config release work currently centers on `config_version` 19. Config schema changes must update bundled defaults, docs, release checks, and migration behavior together.
- GUI regressions can come from overlay resize, pane split/removal, macOS worker thread lifetime, WebGPU surface reconfigure, tab bar spacing, and alternate-screen wheel scroll behavior.
- Startup perf work depends on caching shell user vars, Lua bytecode, early appearance queries, GLSL version, and built-in fonts. Avoid invalidating those caches without a measured reason.
- Notification actions that call back into Kaku should resolve bundled executables relative to the running app, not an assumed system path.

## Release Notes

- Preferred release tag format is uppercase `V0.x.x`.
- `scripts/release.sh` is the source of truth for the release flow.
- The GitHub Release title should come from `.github/RELEASE_NOTES.md`.
- Recovery should be idempotent when a tag already exists and points to the intended commit.

## Documentation Maintenance

- Single-crate behavior belongs in that crate's `AGENTS.md`.
- Cross-crate behavior should update every affected subsystem guide.
- Build or workflow changes belong in this root file.
- Shared instructions belong in `AGENTS.md`; personal overrides belong in ignored local files.
