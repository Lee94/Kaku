//! win32 `Window` implementation.
//!
//! This is the initial bring-up of the Windows window backend, restored/adapted
//! from the upstream WezTerm win32 backend to Kaku's diverged `WindowOps` trait.
//! It creates a real top-level window, runs a `WndProc`, and routes the core
//! lifecycle events (paint/resize/close/destroy) through the shared
//! `WindowEventSender`. Rendering uses the WebGpu front-end via the
//! `raw-window-handle` impls below; `enable_opengl` intentionally errors so the
//! OpenGL/ANGLE path is not yet required (set `front_end = "WebGpu"`).
//!
//! Not yet implemented (tracked as later phases): keyboard/mouse/IME input
//! translation, native clipboard, per-monitor DPI change handling, fullscreen,
//! window-level/position control, and the OpenGL context.

#![allow(clippy::let_unit_value)]

use super::keycodes;
use crate::connection::ConnectionOps;
use crate::{
    Clipboard, ClipboardData, Connection, Dimensions, KeyCode, KeyEvent, KeyboardLedStatus,
    Modifiers, MouseCursor, MouseEvent, MouseEventKind, MousePress, Point, RequestedWindowGeometry,
    ResolvedGeometry, ScreenPoint, WindowEvent, WindowEventSender, WindowOps, WindowState,
};
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use config::ConfigHandle;
use promise::Future;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use std::any::Any;
use std::ffi::OsStr;
use std::io::Error as IoError;
use std::num::NonZeroIsize;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::sync::Once;
use wezterm_font::FontConfiguration;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::*;

const WINDOW_CLASS_NAME: &str = "KakuWindowClass";

/// Encode a Rust string as a NUL-terminated wide (UTF-16) string for win32 APIs.
fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Owns the native window handle plus the per-window event sender. Kept on the
/// GUI thread inside `Connection::windows`; never sent across threads, so the
/// raw `HWND` does not need to be `Send`.
pub(crate) struct WindowInner {
    pub(crate) hwnd: HWND,
    pub(crate) events: WindowEventSender,
    pub(crate) dimensions: Dimensions,
    #[allow(dead_code)]
    pub(crate) window_id: usize,
}

/// Public, cheaply-cloneable handle to a window, addressed by id (mirrors the
/// macОS backend so the rest of the crate is platform-agnostic).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Window {
    id: usize,
}

#[cfg(test)]
impl Window {
    pub(crate) fn for_test(id: usize) -> Self {
        Self { id }
    }
}

impl Window {
    fn hwnd(&self) -> Option<HWND> {
        let conn = Connection::get()?;
        let inner = conn.window_by_id(self.id)?;
        let hwnd = inner.borrow().hwnd;
        Some(hwnd)
    }

    pub async fn new_window<F>(
        _class_name: &str,
        name: &str,
        geometry: RequestedWindowGeometry,
        _config: Option<&ConfigHandle>,
        _font_config: Rc<FontConfiguration>,
        event_handler: F,
    ) -> anyhow::Result<Window>
    where
        F: 'static + FnMut(WindowEvent, &Window),
    {
        let conn = Connection::get().ok_or_else(|| anyhow!("new_window called without a connection"))?;

        let ResolvedGeometry {
            width,
            height,
            x,
            y,
        } = conn.resolve_geometry(geometry);

        register_class();

        let class_name = wide(WINDOW_CLASS_NAME);
        let title = wide(name);
        let hinstance = unsafe { GetModuleHandleW(null()) };

        // Convert the requested client-area size into an overall window size.
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        unsafe {
            AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
        }
        let outer_w = rect.right - rect.left;
        let outer_h = rect.bottom - rect.top;
        let pos_x = x.unwrap_or(CW_USEDEFAULT);
        let pos_y = y.unwrap_or(CW_USEDEFAULT);

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                pos_x,
                pos_y,
                outer_w,
                outer_h,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            )
        };
        if hwnd.is_null() {
            bail!("CreateWindowExW failed: {}", IoError::last_os_error());
        }

        let window_id = conn.next_window_id();
        let events = WindowEventSender::new(event_handler);
        let inner = Rc::new(std::cell::RefCell::new(WindowInner {
            hwnd,
            events: events.clone(),
            dimensions: Dimensions {
                pixel_width: width,
                pixel_height: height,
                dpi: conn.default_dpi() as usize,
            },
            window_id,
        }));
        conn.windows.borrow_mut().insert(window_id, inner);

        // Associate the window id with the HWND so WndProc can route messages.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_id as isize);
        }

        let window = Window { id: window_id };
        events.assign_window(window.clone());
        Ok(window)
    }
}

fn register_class() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| unsafe {
        let class_name = wide(WINDOW_CLASS_NAME);
        let hinstance = GetModuleHandleW(null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: null_mut(),
        };
        RegisterClassExW(&wc);
    });
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let window_id = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
    if window_id == 0 {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let Some(conn) = Connection::get() else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    let Some(inner_rc) = conn.window_by_id(window_id) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };
    // Clone the sender so we don't hold a borrow across the (possibly reentrant)
    // event dispatch into the GUI callback.
    let events = inner_rc.borrow().events.clone();

    match msg {
        WM_PAINT => {
            ValidateRect(hwnd, null());
            events.dispatch(WindowEvent::NeedRepaint);
            0
        }
        WM_SIZE => {
            let width = (lparam & 0xffff) as usize;
            let height = ((lparam >> 16) & 0xffff) as usize;
            let dpi = crate::DEFAULT_DPI as usize;
            let dimensions = Dimensions {
                pixel_width: width,
                pixel_height: height,
                dpi,
            };
            inner_rc.borrow_mut().dimensions = dimensions;
            events.dispatch(WindowEvent::Resized {
                dimensions,
                window_state: WindowState::empty(),
                live_resizing: wparam == SIZE_RESTORED as WPARAM,
                screen_changed: false,
            });
            0
        }
        WM_SETFOCUS => {
            events.dispatch(WindowEvent::FocusChanged(true));
            0
        }
        WM_KILLFOCUS => {
            events.dispatch(WindowEvent::FocusChanged(false));
            0
        }
        WM_CHAR => {
            // WM_CHAR delivers the layout-translated character, including control
            // chars for Ctrl combos (e.g. Ctrl+C -> 0x03). The modifiers are
            // already baked into the char, so report NONE here.
            if let Some(c) = char::from_u32(wparam as u32) {
                if c != '\0' {
                    events.dispatch(WindowEvent::KeyEvent(KeyEvent {
                        key: KeyCode::Char(c),
                        modifiers: Modifiers::NONE,
                        leds: KeyboardLedStatus::empty(),
                        repeat_count: 1,
                        key_is_down: true,
                        raw: None,
                        win32_uni_char: Some(c),
                    }));
                }
            }
            0
        }
        WM_KEYDOWN => {
            // Navigation/function keys don't produce WM_CHAR; translate them
            // here. Text keys fall through to DefWindowProc -> WM_CHAR.
            if let Some(key) = keycodes::vkey_to_keycode(wparam as i32) {
                events.dispatch(WindowEvent::KeyEvent(KeyEvent {
                    key,
                    modifiers: keycodes::current_modifiers(),
                    leds: KeyboardLedStatus::empty(),
                    repeat_count: (lparam & 0xffff) as u16,
                    key_is_down: true,
                    raw: None,
                    win32_uni_char: None,
                }));
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Move,
                wparam,
                lparam,
            )));
            0
        }
        WM_LBUTTONDOWN => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Press(MousePress::Left),
                wparam,
                lparam,
            )));
            0
        }
        WM_LBUTTONUP => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Release(MousePress::Left),
                wparam,
                lparam,
            )));
            0
        }
        WM_RBUTTONDOWN => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Press(MousePress::Right),
                wparam,
                lparam,
            )));
            0
        }
        WM_RBUTTONUP => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Release(MousePress::Right),
                wparam,
                lparam,
            )));
            0
        }
        WM_MBUTTONDOWN => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Press(MousePress::Middle),
                wparam,
                lparam,
            )));
            0
        }
        WM_MBUTTONUP => {
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::Release(MousePress::Middle),
                wparam,
                lparam,
            )));
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) & 0xffff) as i16;
            events.dispatch(WindowEvent::MouseEvent(make_mouse(
                MouseEventKind::VertWheel(delta / 120),
                wparam,
                lparam,
            )));
            0
        }
        WM_CLOSE => {
            events.dispatch(WindowEvent::CloseRequested);
            0
        }
        WM_DESTROY => {
            events.dispatch(WindowEvent::Destroyed);
            conn.windows.borrow_mut().remove(&window_id);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Build a `MouseEvent` from a mouse message's `wparam`/`lparam`.
///
/// Note: for `WM_MOUSEWHEEL`, `lparam` holds screen (not client) coordinates;
/// that is a minor inaccuracy for hit-testing the wheel target and can be
/// refined with `ScreenToClient` later.
fn make_mouse(kind: MouseEventKind, wparam: WPARAM, lparam: LPARAM) -> MouseEvent {
    let x = (lparam & 0xffff) as i16 as isize;
    let y = ((lparam >> 16) & 0xffff) as i16 as isize;
    MouseEvent {
        kind,
        coords: Point::new(x, y),
        screen_coords: ScreenPoint::new(x, y),
        mouse_buttons: keycodes::mouse_buttons_from_wparam(wparam),
        modifiers: keycodes::current_modifiers(),
        platform_click_count: 0,
    }
}

#[async_trait(?Send)]
impl WindowOps for Window {
    async fn enable_opengl(&self) -> anyhow::Result<Rc<glium::backend::Context>> {
        bail!("the OpenGL front_end is not yet implemented on Windows; set front_end = \"WebGpu\"")
    }

    fn notify<T: Any + Send + Sync>(&self, t: T)
    where
        Self: Sized,
    {
        Connection::with_window_inner(self.id, move |inner| {
            inner
                .events
                .dispatch(WindowEvent::Notification(Box::new(t)));
            Ok(())
        });
    }

    fn show(&self) {
        Connection::with_window_inner(self.id, |inner| {
            unsafe {
                ShowWindow(inner.hwnd, SW_SHOW);
            }
            Ok(())
        });
    }

    fn hide(&self) {
        Connection::with_window_inner(self.id, |inner| {
            unsafe {
                ShowWindow(inner.hwnd, SW_HIDE);
            }
            Ok(())
        });
    }

    fn close(&self) {
        Connection::with_window_inner(self.id, |inner| {
            unsafe {
                DestroyWindow(inner.hwnd);
            }
            Ok(())
        });
    }

    fn focus(&self) {
        Connection::with_window_inner(self.id, |inner| {
            unsafe {
                SetForegroundWindow(inner.hwnd);
            }
            Ok(())
        });
    }

    fn set_cursor(&self, _cursor: Option<MouseCursor>) {
        // TODO(windows): translate MouseCursor -> LoadCursorW and SetCursor.
    }

    fn invalidate(&self) {
        Connection::with_window_inner(self.id, |inner| {
            unsafe {
                InvalidateRect(inner.hwnd, null(), 0);
            }
            Ok(())
        });
    }

    fn set_title(&self, title: &str) {
        let title = title.to_owned();
        Connection::with_window_inner(self.id, move |inner| {
            let wide_title = wide(&title);
            unsafe {
                SetWindowTextW(inner.hwnd, wide_title.as_ptr());
            }
            Ok(())
        });
    }

    fn set_inner_size(&self, width: usize, height: usize) {
        Connection::with_window_inner(self.id, move |inner| {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            unsafe {
                AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
                SetWindowPos(
                    inner.hwnd,
                    null_mut(),
                    0,
                    0,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            Ok(())
        });
    }

    fn get_clipboard(&self, _clipboard: Clipboard) -> Future<String> {
        // TODO(windows): read CF_UNICODETEXT from the native clipboard.
        Future::ok(String::new())
    }

    fn get_clipboard_data(&self, _clipboard: Clipboard) -> Future<ClipboardData> {
        Future::ok(ClipboardData::Text(String::new()))
    }

    fn set_clipboard(&self, _clipboard: Clipboard, _text: String) {
        // TODO(windows): write CF_UNICODETEXT to the native clipboard.
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Windows(
                WindowsDisplayHandle::new(),
            )))
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = self.hwnd().ok_or(HandleError::Unavailable)?;
        let mut handle = Win32WindowHandle::new(
            NonZeroIsize::new(hwnd as isize).ok_or(HandleError::Unavailable)?,
        );
        let hinstance = unsafe { GetWindowLongPtrW(hwnd, GWLP_HINSTANCE) };
        handle.hinstance = NonZeroIsize::new(hinstance);
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
    }
}
