//! Windows (win32) backend for the `window` crate.
//!
//! This backend is being restored from the upstream WezTerm win32 implementation
//! and adapted to Kaku's diverged `ConnectionOps` / `WindowOps` traits as part of
//! the macOS -> Windows port. It is brought up incrementally:
//!
//!   * Phase 0 (done): module wiring + low-level primitives that the shared,
//!     non-platform code already references (`event::EventHandle` used by
//!     `spawn.rs`, and `is_running_in_rdp_session` used by `configuration.rs`).
//!   * Phase 1 (in progress): `connection` (message loop, screen/DPI, appearance)
//!     and `window` (HWND lifecycle, input, clipboard, `enable_opengl`,
//!     raw-window-handle) which provide the `Connection` and `Window` types this
//!     module must re-export, mirroring `os::macos`.
//!
//! Until Phase 1 lands, the `window` crate does not fully compile for Windows.
//! Everything here is behind `#[cfg(windows)]`, so the macOS build is unaffected.

pub mod connection;
pub mod event;
pub mod keycodes;
pub mod window;

pub use self::connection::*;
pub use self::window::*;

// Later refinement: native clipboard (currently stubbed in window.rs).
// pub mod clipboard;

/// Returns true when the current process is running inside a Remote Desktop
/// (RDP) session. Used by `configuration.rs` to fall back to software rendering,
/// because hardware OpenGL/Direct3D contexts behave poorly across RDP
/// connect/disconnect cycles.
pub fn is_running_in_rdp_session() -> bool {
    // SM_REMOTESESSION is non-zero when the calling process is associated with
    // a Terminal Services / RDP client session.
    unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_REMOTESESSION) != 0 }
}
