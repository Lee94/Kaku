use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use config::keyassignment::{ClipboardCopyDestination, ClipboardPasteSource};
use mux::pane::Pane;
use mux::Mux;
use std::path::PathBuf;
use std::sync::Arc;
use window::{Clipboard, ClipboardData, WindowOps};

impl TermWindow {
    pub fn copy_to_clipboard(&self, clipboard: ClipboardCopyDestination, text: String) {
        let clipboard = match clipboard {
            ClipboardCopyDestination::Clipboard => [Some(Clipboard::Clipboard), None],
            ClipboardCopyDestination::PrimarySelection => [Some(Clipboard::PrimarySelection), None],
            ClipboardCopyDestination::ClipboardAndPrimarySelection => [
                Some(Clipboard::Clipboard),
                Some(Clipboard::PrimarySelection),
            ],
        };
        for &c in &clipboard {
            if let Some(c) = c {
                self.window.as_ref().unwrap().set_clipboard(c, text.clone());
            }
        }
    }

    pub fn paste_from_clipboard(&mut self, pane: &Arc<dyn Pane>, clipboard: ClipboardPasteSource) {
        let pane_id = pane.pane_id();
        log::trace!(
            "paste_from_clipboard in pane {} {:?}",
            pane.pane_id(),
            clipboard
        );
        let window = self.window.as_ref().unwrap().clone();
        let clipboard = match clipboard {
            ClipboardPasteSource::Clipboard => Clipboard::Clipboard,
            ClipboardPasteSource::PrimarySelection => Clipboard::PrimarySelection,
        };
        let quote_dropped_files = self.config.quote_dropped_files;
        let future = window.get_clipboard_data(clipboard);
        promise::spawn::spawn(async move {
            match future.await {
                Ok(data) => {
                    window.notify(TermWindowNotif::Apply(Box::new(move |myself| {
                        let clip = match data_to_paste_string(data, quote_dropped_files) {
                            Some(clip) => clip,
                            None => return,
                        };

                        if let Some(pane) = myself
                            .pane_state(pane_id)
                            .overlay
                            .as_ref()
                            .map(|overlay| overlay.pane.clone())
                            .or_else(|| {
                                let mux = Mux::get();
                                mux.get_pane(pane_id)
                            })
                        {
                            if let Err(err) = pane.send_paste(&clip) {
                                log::warn!("failed to paste clipboard content into pane {pane_id}: {err:#}");
                            }
                        }
                    })));
                }
                Err(err) => {
                    log::warn!("failed to read clipboard for pane {pane_id}: {err:#}");
                }
            }
        })
        .detach();
        self.maybe_scroll_to_bottom_for_input(&pane);
    }
}

fn data_to_paste_string(
    data: ClipboardData,
    quote_dropped_files: config::DroppedFileQuoting,
) -> Option<String> {
    match data {
        ClipboardData::Text(text) => Some(text),
        ClipboardData::Files(paths) => {
            if paths.is_empty() {
                return None;
            }
            Some(format_dropped_paths(paths, quote_dropped_files))
        }
    }
}

fn format_dropped_paths(
    paths: Vec<PathBuf>,
    quote_dropped_files: config::DroppedFileQuoting,
) -> String {
    paths
        .iter()
        .map(|path| quote_dropped_files.escape(&path.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
        + " "
}
