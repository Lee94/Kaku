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

    /// Toggle the first window's visibility, used by the global hotkey: hide it
    /// when it is the foreground window, otherwise show and focus it.
    fn toggle_main_window(&self) {
        let windows: Vec<_> = self.windows.borrow().values().cloned().collect();
        if let Some(inner) = windows.first() {
            let hwnd = inner.borrow().hwnd;
            unsafe {
                if IsWindowVisible(hwnd) != 0 && GetForegroundWindow() == hwnd {
                    ShowWindow(hwnd, SW_HIDE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                    SetForegroundWindow(hwnd);
                }
            }
        }
    }
}

/// The real system DPI (e.g. 192 at 200% scale), or DEFAULT_DPI if unavailable.
/// The process is per-monitor DPI aware (manifest), so reporting the true DPI is
/// what makes the GUI render fonts at the correct, crisp scale on HiDPI displays.
pub(crate) fn system_dpi() -> f64 {
    let dpi = unsafe { GetDpiForSystem() };
    if dpi == 0 {
        crate::DEFAULT_DPI
    } else {
        dpi as f64
    }
}

impl ConnectionOps for Connection {
    fn name(&self) -> String {
        "Windows".to_string()
    }

    fn default_dpi(&self) -> f64 {
        system_dpi()
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
                    // RegisterHotKey delivers WM_HOTKEY to the thread queue with a
                    // null hwnd, so it isn't routed to any WndProc; handle it here.
                    if msg.message == WM_HOTKEY {
                        self.toggle_main_window();
                        continue;
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

    fn sync_global_hotkey(&self) {
        unsafe {
            UnregisterHotKey(std::ptr::null_mut(), GLOBAL_HOTKEY_ID);
        }
        if let Some((vk, mods)) = configured_global_hotkey() {
            let ok = unsafe {
                RegisterHotKey(
                    std::ptr::null_mut(),
                    GLOBAL_HOTKEY_ID,
                    mods | MOD_NOREPEAT as u32,
                    vk,
                )
            };
            if ok == 0 {
                log::warn!(
                    "RegisterHotKey failed for the global hotkey (vk={vk}, mods={mods:#x}); \
                     it may be reserved by the system. Set config.macos_global_hotkey to a \
                     free combo such as {{ key = 'K', mods = 'CTRL|ALT' }}."
                );
            }
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
        let dpi = system_dpi();
        let info = ScreenInfo {
            name: "primary".to_string(),
            rect,
            scale: dpi / crate::DEFAULT_DPI,
            max_fps: None,
            effective_dpi: Some(dpi),
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

const GLOBAL_HOTKEY_ID: i32 = 1;

/// Resolve the configured global hotkey to (virtual key, MOD_* flags), or None.
/// Reuses the `macos_global_hotkey` config field (the only global-hotkey setting
/// today); a modifier is required so we never bind a bare key globally.
fn configured_global_hotkey() -> Option<(u32, u32)> {
    let config = config::configuration();
    let hotkey = config.macos_global_hotkey.clone()?;
    let key = hotkey.key.resolve(config.key_map_preference);
    let vk = super::keycodes::keycode_to_vk(&key)?;
    let mods = super::keycodes::mods_to_win32(hotkey.mods.remove_positional_mods());
    if mods == 0 {
        return None;
    }
    Some((vk, mods))
}
