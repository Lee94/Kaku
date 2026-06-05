use crate::ToastNotification;

/// Minimal Windows notification backend.
///
/// TODO(windows port, Phase 6): replace with native WinRT toast notifications
/// (`ToastNotificationManager`), including the click-to-open-url action that
/// resolves the bundled executable relative to the running app (mirroring the
/// macOS backend). For now we log the notification so it is observable rather
/// than silently dropped, which is enough to bring the GUI up on Windows.
pub fn show_notif(toast: ToastNotification) -> Result<(), Box<dyn std::error::Error>> {
    match &toast.url {
        Some(url) => log::info!(
            "notification: {} - {} ({})",
            toast.title,
            toast.message,
            url
        ),
        None => log::info!("notification: {} - {}", toast.title, toast.message),
    }
    Ok(())
}
