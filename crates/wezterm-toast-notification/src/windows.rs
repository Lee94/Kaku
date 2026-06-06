//! Windows toast notifications via WinRT (`winrt-notification`).
//!
//! Click-to-open-url actions are not yet wired (that needs COM activation
//! registration); for now the toast shows the title and message, attributed to
//! Kaku's AppUserModelID. Real activation handling is a later enhancement.

use crate::ToastNotification;
use winrt_notification::{Duration as ToastDuration, Toast};

/// Must match the AppUserModelID set in kaku-gui via
/// `SetCurrentProcessExplicitAppUserModelID` so toasts are attributed to Kaku.
const APP_ID: &str = "sh.kaku.Kaku";

pub fn show_notif(toast: ToastNotification) -> Result<(), Box<dyn std::error::Error>> {
    let duration = match toast.timeout {
        Some(d) if d.as_secs() >= 9 => ToastDuration::Long,
        _ => ToastDuration::Short,
    };
    Toast::new(APP_ID)
        .title(&toast.title)
        .text1(&toast.message)
        .duration(duration)
        .show()?;
    Ok(())
}
