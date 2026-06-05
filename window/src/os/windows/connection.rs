//! win32 `Connection`: the per-thread application/event-loop object.
//!
//! Mirrors the macОS `Connection` structure so the rest of the crate stays
//! platform-agnostic. The message loop integrates the cross-thread spawn queue
//! (woken via `SPAWN_QUEUE.event_handle`) with the native win32 message pump.

use super::window::WindowInner;
use crate::connection::ConnectionOps;
use crate::screen::{ScreenInfo, Screens};
use crate::spawn::*;
use crate::Appearance;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use winapi::um::winbase::INFINITE;
use winapi::um::winuser::*;

pub struct Connection {
    pub(crate) windows: RefCell<HashMap<usize, Rc<RefCell<WindowInner>>>>,
    pub(crate) next_window_id: AtomicUsize,
}

impl Connection {
    pub(crate) fn create_new() -> anyhow::Result<Self> {
        // Ensure the SPAWN_QUEUE is created; nothing to run yet.
        SPAWN_QUEUE.run();
        Ok(Self {
            windows: RefCell::new(HashMap::new()),
            next_window_id: AtomicUsize::new(1),
        })
    }

    pub(crate) fn next_window_id(&self) -> usize {
        self.next_window_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn window_by_id(&self, window_id: usize) -> Option<Rc<RefCell<WindowInner>>> {
        self.windows.borrow().get(&window_id).map(Rc::clone)
    }

    // Retained to mirror the macOS backend; window ops currently copy the HWND
    // out under a brief borrow instead (see window.rs) to avoid re-entrant
    // RefCell borrows when win32 calls synchronously dispatch messages.
    #[allow(dead_code)]
    pub(crate) fn with_window_inner<
        R,
        F: FnOnce(&mut WindowInner) -> anyhow::Result<R> + Send + 'static,
    >(
        window_id: usize,
        f: F,
    ) -> promise::Future<R>
    where
        R: Send + 'static,
    {
        let mut prom = promise::Promise::new();
        let future = prom.get_future().unwrap();
        promise::spawn::spawn_into_main_thread(async move {
            let result = match Connection::get() {
                Some(conn) => match conn.window_by_id(window_id) {
                    Some(handle) => {
                        let mut inner = handle.borrow_mut();
                        f(&mut inner)
                    }
                    None => Err(anyhow::anyhow!("invalid window id {}", window_id)),
                },
                None => Err(anyhow::anyhow!("window connection is not initialized")),
            };
            prom.result(result);
        })
        .detach();
        future
    }
}

impl ConnectionOps for Connection {
    fn name(&self) -> String {
        "Windows".to_string()
    }

    fn terminate_message_loop(&self) {
        unsafe {
            PostQuitMessage(0);
        }
    }

    fn run_message_loop(&self) -> anyhow::Result<()> {
        unsafe {
            let spawn_handle = SPAWN_QUEUE.event_handle.handle();
            let mut msg: MSG = std::mem::zeroed();
            loop {
                // Run any work queued from other threads / the async runtime.
                SPAWN_QUEUE.run();

                // Drain all pending native messages.
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        self.windows.borrow_mut().clear();
                        return Ok(());
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                // Run again to catch work enqueued while dispatching messages.
                SPAWN_QUEUE.run();

                // Sleep until either a native message arrives or the spawn queue
                // signals its event handle.
                let handles = [spawn_handle];
                MsgWaitForMultipleObjects(
                    handles.len() as u32,
                    handles.as_ptr(),
                    0, // bWaitAll = FALSE
                    INFINITE,
                    QS_ALLINPUT,
                );
            }
        }
    }

    fn get_appearance(&self) -> Appearance {
        match read_apps_use_light_theme() {
            Some(false) => Appearance::Dark,
            _ => Appearance::Light,
        }
    }

    fn beep(&self) {
        unsafe {
            MessageBeep(0xFFFF_FFFF);
        }
    }

    fn screens(&self) -> anyhow::Result<Screens> {
        // Initial bring-up: report a single primary screen. Per-monitor
        // enumeration (EnumDisplayMonitors) and DPI is a later refinement.
        let (w, h) = unsafe {
            (
                GetSystemMetrics(SM_CXSCREEN) as isize,
                GetSystemMetrics(SM_CYSCREEN) as isize,
            )
        };
        let rect = euclid::rect(0, 0, w, h);
        let info = ScreenInfo {
            name: "primary".to_string(),
            rect,
            scale: 1.0,
            max_fps: None,
            effective_dpi: Some(crate::DEFAULT_DPI),
        };
        let mut by_name = HashMap::new();
        by_name.insert(info.name.clone(), info.clone());
        Ok(Screens {
            main: info.clone(),
            active: info.clone(),
            by_name,
            virtual_rect: rect,
        })
    }
}

/// Read the system "apps use light theme" preference from the registry.
/// Returns `Some(true)` for light, `Some(false)` for dark, `None` if unset.
fn read_apps_use_light_theme() -> Option<bool> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .ok()?;
    let value: u32 = key.get_value("AppsUseLightTheme").ok()?;
    Some(value != 0)
}
