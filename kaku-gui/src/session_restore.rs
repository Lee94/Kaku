use crate::termwindow::TermWindowNotif;
use crate::{frontend, TermWindow};
use anyhow::{anyhow, Context};
use config::GuiPosition;
use mux::pane::{Pane, PaneId};
use mux::tab::{PaneEntry, PaneNode, SerdeUrl, SplitDirectionAndSize, Tab, TabId};
use mux::window::WindowId as MuxWindowId;
use mux::Mux;
use parking_lot::Mutex;
use promise::spawn::spawn;
use serde::{Deserialize, Serialize};
use smol::Timer;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use wezterm_term::TerminalSize;
use wezterm_toast_notification::persistent_toast_notification;
use window::WindowOps;

const SNAPSHOT_VERSION: u32 = 2;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Serialize, Deserialize)]
struct SavedWindowSnapshot {
    version: u32,
    active_tab_idx: usize,
    window_title: String,
    tabs: Vec<SavedTabSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedTabSnapshot {
    title: String,
    pane_tree: SavedPaneNode,
}

#[derive(Debug, Serialize, Deserialize)]
enum SavedPaneNode {
    Empty,
    Split {
        left: Box<SavedPaneNode>,
        right: Box<SavedPaneNode>,
        node: SplitDirectionAndSize,
    },
    Leaf(SavedPaneEntry),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SavedPaneEntry {
    window_id: MuxWindowId,
    tab_id: TabId,
    pane_id: PaneId,
    title: String,
    size: TerminalSize,
    working_dir: Option<SerdeUrl>,
    domain_name: String,
    is_active_pane: bool,
    is_zoomed_pane: bool,
    workspace: String,
}

impl SavedPaneNode {
    fn from_live(node: PaneNode, mux: &Mux) -> anyhow::Result<Self> {
        match node {
            PaneNode::Empty => Ok(Self::Empty),
            PaneNode::Split { left, right, node } => Ok(Self::Split {
                left: Box::new(Self::from_live(*left, mux)?),
                right: Box::new(Self::from_live(*right, mux)?),
                node,
            }),
            PaneNode::Leaf(entry) => Ok(Self::Leaf(SavedPaneEntry::from_live(entry, mux)?)),
        }
    }

    fn root_size(&self) -> Option<TerminalSize> {
        match self {
            Self::Empty => None,
            Self::Split { node, .. } => Some(node.size()),
            Self::Leaf(entry) => Some(entry.size),
        }
    }

    fn into_pane_node(self) -> PaneNode {
        match self {
            Self::Empty => PaneNode::Empty,
            Self::Split { left, right, node } => PaneNode::Split {
                left: Box::new(left.into_pane_node()),
                right: Box::new(right.into_pane_node()),
                node,
            },
            Self::Leaf(entry) => PaneNode::Leaf(entry.into_pane_entry()),
        }
    }
}

impl SavedPaneEntry {
    fn from_live(entry: PaneEntry, mux: &Mux) -> anyhow::Result<Self> {
        let pane = mux
            .get_pane(entry.pane_id)
            .ok_or_else(|| anyhow!("pane {} not found while building snapshot", entry.pane_id))?;
        let domain = mux.get_domain(pane.domain_id()).ok_or_else(|| {
            anyhow!(
                "domain {} not found while building snapshot for pane {}",
                pane.domain_id(),
                entry.pane_id
            )
        })?;

        Ok(Self {
            window_id: entry.window_id,
            tab_id: entry.tab_id,
            pane_id: entry.pane_id,
            title: entry.title,
            size: entry.size,
            working_dir: entry.working_dir,
            domain_name: domain.domain_name().to_string(),
            is_active_pane: entry.is_active_pane,
            is_zoomed_pane: entry.is_zoomed_pane,
            workspace: entry.workspace,
        })
    }

    fn into_pane_entry(self) -> PaneEntry {
        PaneEntry {
            window_id: self.window_id,
            tab_id: self.tab_id,
            pane_id: self.pane_id,
            title: self.title,
            size: self.size,
            working_dir: self.working_dir,
            is_active_pane: self.is_active_pane,
            is_zoomed_pane: self.is_zoomed_pane,
            workspace: self.workspace,
            cursor_pos: Default::default(),
            physical_top: 0,
            top_row: 0,
            left_col: 0,
            tty_name: None,
        }
    }
}

fn config_dir_file(name: &str) -> PathBuf {
    config::CONFIG_DIRS
        .first()
        .cloned()
        .unwrap_or_else(|| config::HOME_DIR.join(".config").join("kaku"))
        .join(name)
}

fn snapshot_file() -> PathBuf {
    config_dir_file("last_window_session.json")
}

fn collect_leaf_entries(node: &SavedPaneNode, out: &mut Vec<SavedPaneEntry>) {
    match node {
        SavedPaneNode::Empty => {}
        SavedPaneNode::Split { left, right, .. } => {
            collect_leaf_entries(left, out);
            collect_leaf_entries(right, out);
        }
        SavedPaneNode::Leaf(entry) => out.push(entry.clone()),
    }
}

fn cwd_from_working_dir(working_dir: Option<&SerdeUrl>) -> Option<String> {
    let url = working_dir?;
    if url.url.scheme() != "file" {
        return None;
    }
    url.url
        .to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn focused_window_id() -> Option<MuxWindowId> {
    frontend::try_front_end()
        .and_then(|fe| fe.focused_mux_window_id())
        .or_else(|| {
            let mux = Mux::get();
            let mut windows = mux.iter_windows();
            windows.sort();
            windows.pop()
        })
}

fn build_snapshot_for_window(
    window_id: MuxWindowId,
) -> anyhow::Result<Option<SavedWindowSnapshot>> {
    let mux = Mux::get();
    let window = mux
        .get_window(window_id)
        .ok_or_else(|| anyhow!("window {window_id} not found"))?;

    let tab_count = window.len();
    let pane_count: usize = window
        .iter()
        .map(|tab| tab.iter_panes_ignoring_zoom().len())
        .sum();

    if tab_count <= 1 && pane_count <= 1 {
        return Ok(None);
    }

    let tabs = window
        .iter()
        .map(|tab| {
            Ok(SavedTabSnapshot {
                title: tab.get_title(),
                pane_tree: SavedPaneNode::from_live(tab.codec_pane_tree(), &mux)?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(Some(SavedWindowSnapshot {
        version: SNAPSHOT_VERSION,
        active_tab_idx: window.get_active_idx(),
        window_title: window.get_title().to_string(),
        tabs,
    }))
}

fn write_snapshot_to_path(
    file_name: &std::path::Path,
    snapshot: &SavedWindowSnapshot,
) -> anyhow::Result<()> {
    if let Some(parent) = file_name.parent() {
        config::create_user_owned_dirs(parent)
            .with_context(|| format!("create snapshot dir {}", parent.display()))?;
    }

    let encoded = serde_json::to_string_pretty(snapshot).context("encode window snapshot")?;
    // Atomic write: a crash mid-write would otherwise leave a truncated JSON
    // file that fails to parse on the next launch. Write to a sibling temp
    // file and rename on top, which is atomic on POSIX.
    let tmp = file_name.with_file_name(format!(
        "{}.{}.{}.tmp",
        file_name.file_stem().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&tmp, format!("{encoded}\n"))
        .with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, &file_name)
        .with_context(|| format!("rename {} -> {}", tmp.display(), file_name.display()))?;
    Ok(())
}

fn write_snapshot(snapshot: &SavedWindowSnapshot) -> anyhow::Result<()> {
    write_snapshot_to_path(&snapshot_file(), snapshot)
}

pub fn save_window_snapshot(window_id: MuxWindowId) -> anyhow::Result<()> {
    let Some(snapshot) = build_snapshot_for_window(window_id)? else {
        return Ok(());
    };

    write_snapshot(&snapshot)
}

pub fn save_focused_window_snapshot() -> anyhow::Result<()> {
    let Some(window_id) = focused_window_id() else {
        return Ok(());
    };

    save_window_snapshot(window_id)
}

fn debounce_state() -> &'static Mutex<HashMap<MuxWindowId, u64>> {
    static STATE: OnceLock<Mutex<HashMap<MuxWindowId, u64>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn request_save_window_snapshot(window_id: MuxWindowId) {
    let generation = {
        let mut map = debounce_state().lock();
        let entry = map.entry(window_id).or_insert(0);
        *entry = entry.wrapping_add(1);
        *entry
    };

    spawn(async move {
        Timer::after(SAVE_DEBOUNCE).await;
        let latest = debounce_state()
            .lock()
            .get(&window_id)
            .copied()
            .unwrap_or(0);
        if latest != generation {
            return;
        }
        if let Err(err) = save_window_snapshot(window_id) {
            log::debug!("failed to save window snapshot for {window_id}: {err:#}");
        }
    })
    .detach();
}

/// Reads the on-disk snapshot.
///
/// Returns `Ok(None)` when there is nothing usable to restore (no file, an
/// older/newer version we don't recognize, a corrupt JSON, or an empty tab
/// list). The menu treats this as a soft "nothing to restore" instead of
/// surfacing a noisy error toast — version bumps would otherwise greet every
/// upgrading user with a failure.
///
/// `Err` is reserved for unexpected I/O errors that the user should know
/// about (e.g. permission denied).
fn load_snapshot_from_path(
    file_name: &std::path::Path,
) -> anyhow::Result<Option<SavedWindowSnapshot>> {
    let contents = match std::fs::read_to_string(&file_name) {
        Ok(s) => s,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!("read {}", file_name.display())));
        }
    };

    let snapshot: SavedWindowSnapshot = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(err) => {
            log::warn!(
                "ignoring corrupt window snapshot at {}: {err}",
                file_name.display()
            );
            return Ok(None);
        }
    };

    if snapshot.version != SNAPSHOT_VERSION {
        log::warn!(
            "ignoring window snapshot at {} with unsupported version {} (expected {})",
            file_name.display(),
            snapshot.version,
            SNAPSHOT_VERSION
        );
        return Ok(None);
    }

    if snapshot.tabs.is_empty() {
        log::debug!("snapshot {} has no tabs", file_name.display());
        return Ok(None);
    }

    Ok(Some(snapshot))
}

fn load_snapshot() -> anyhow::Result<Option<SavedWindowSnapshot>> {
    load_snapshot_from_path(&snapshot_file())
}

async fn spawn_panes_for_tab(
    root: &SavedPaneNode,
) -> anyhow::Result<HashMap<PaneId, Arc<dyn Pane>>> {
    let mux = Mux::get();
    let encoding = config::configuration().default_encoding;
    let mut entries = Vec::new();
    collect_leaf_entries(root, &mut entries);

    let mut panes = HashMap::new();
    for entry in entries {
        let domain = mux
            .get_domain_by_name(&entry.domain_name)
            .ok_or_else(|| anyhow!("snapshot domain `{}` is not available", entry.domain_name))?;
        let pane = domain
            .spawn_pane(
                &mux,
                entry.size,
                None,
                cwd_from_working_dir(entry.working_dir.as_ref()),
                encoding,
            )
            .await
            .with_context(|| {
                format!(
                    "spawn pane for snapshot pane {} in domain `{}`",
                    entry.pane_id, entry.domain_name
                )
            })?;
        panes.insert(entry.pane_id, pane);
    }

    Ok(panes)
}

async fn get_existing_terminal_size() -> Option<TerminalSize> {
    let window = frontend::try_front_end()?
        .gui_windows()
        .into_iter()
        .next()
        .map(|w| w.window.clone())?;
    let (tx, rx) = smol::channel::bounded::<TerminalSize>(1);
    window.notify(TermWindowNotif::Apply(Box::new(
        move |tw: &mut TermWindow| {
            let _ = tx.try_send(tw.get_terminal_size());
        },
    )));
    rx.recv().await.ok()
}

async fn restore_snapshot(snapshot: SavedWindowSnapshot) -> anyhow::Result<()> {
    let mux = Mux::get();
    let SavedWindowSnapshot {
        version: _,
        active_tab_idx,
        window_title,
        tabs,
    } = snapshot;
    let workspace = mux.active_workspace();

    let builder = mux.new_empty_window(Some(workspace), None::<GuiPosition>);
    let window_id = *builder;

    let actual_size = get_existing_terminal_size().await;

    let restore_result = async {
        for saved_tab in tabs {
            let size = saved_tab.pane_tree.root_size().unwrap_or_default();
            let tab = Arc::new(Tab::new(&size));
            let panes = spawn_panes_for_tab(&saved_tab.pane_tree).await?;
            let pane_tree = saved_tab.pane_tree.into_pane_node();

            tab.try_sync_with_pane_tree(size, pane_tree, |entry| {
                panes
                    .get(&entry.pane_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("missing restored pane {}", entry.pane_id))
            })?;

            if !saved_tab.title.is_empty() {
                tab.set_title(&saved_tab.title);
            }

            mux.add_tab_no_panes(&tab);
            mux.add_tab_to_window(&tab, window_id)?;

            if let Some(s) = actual_size {
                tab.resize(s);
            }
        }

        if let Some(mut window) = mux.get_window_mut(window_id) {
            if !window_title.is_empty() {
                window.set_title(&window_title);
            }
            if window.len() > 0 {
                let max_idx = window.len() - 1;
                window.set_active_without_saving(active_tab_idx.min(max_idx));
            }
        }

        Ok::<(), anyhow::Error>(())
    }
    .await;

    match restore_result {
        Ok(()) => {
            drop(builder);
            Ok(())
        }
        Err(err) => {
            builder.cancel();
            Err(err)
        }
    }
}

#[allow(dead_code)]
pub fn restore_previous_window_from_menu() {
    spawn(async move {
        let result = async {
            match load_snapshot()? {
                Some(snapshot) => {
                    restore_snapshot(snapshot).await?;
                    Ok::<bool, anyhow::Error>(true)
                }
                None => Ok(false),
            }
        }
        .await;

        match result {
            Ok(true) => {}
            Ok(false) => {
                persistent_toast_notification(
                    "Restore Previous Window",
                    "No previous window snapshot is available to restore.",
                );
            }
            Err(err) => {
                log::warn!("failed to restore previous window: {err:#}");
                persistent_toast_notification("Restore Previous Window", &format!("{err:#}"));
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot(version: u32) -> SavedWindowSnapshot {
        SavedWindowSnapshot {
            version,
            active_tab_idx: 0,
            window_title: "Test Window".to_string(),
            tabs: vec![SavedTabSnapshot {
                title: "Test Tab".to_string(),
                pane_tree: SavedPaneNode::Empty,
            }],
        }
    }

    #[test]
    fn snapshot_round_trips_via_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_window_session.json");
        write_snapshot_to_path(&path, &sample_snapshot(SNAPSHOT_VERSION)).unwrap();

        let loaded = load_snapshot_from_path(&path).unwrap().expect("snapshot");
        assert_eq!(loaded.version, SNAPSHOT_VERSION);
        assert_eq!(loaded.active_tab_idx, 0);
        assert_eq!(loaded.window_title, "Test Window");
        assert_eq!(loaded.tabs.len(), 1);
        assert_eq!(loaded.tabs[0].title, "Test Tab");
    }

    #[test]
    fn corrupt_snapshot_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_window_session.json");
        std::fs::write(&path, "{not json").unwrap();

        assert!(load_snapshot_from_path(&path).unwrap().is_none());
    }

    #[test]
    fn unsupported_snapshot_version_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_window_session.json");
        std::fs::write(
            &path,
            serde_json::to_string(&sample_snapshot(SNAPSHOT_VERSION + 1)).unwrap(),
        )
        .unwrap();

        assert!(load_snapshot_from_path(&path).unwrap().is_none());
    }
}
