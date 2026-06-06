use std::io::Error as IoError;
use std::ptr::{null, null_mut};
use winapi::shared::ntdef::HANDLE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::synchapi::{CreateEventW, ResetEvent, SetEvent};

/// A manual-reset Win32 event object. The GUI message loop waits on this handle
/// (via `MsgWaitForMultipleObjects`) so that work pushed onto the spawn queue
/// from another thread wakes the loop promptly. This mirrors the wakeup pipe
/// used by the unix backend and the run-loop observer used by macOS.
pub struct EventHandle {
    handle: HANDLE,
}

// A Win32 event HANDLE is safe to signal/reset/wait on from any thread.
unsafe impl Send for EventHandle {}
unsafe impl Sync for EventHandle {}

impl EventHandle {
    /// Create a manual-reset event in the non-signaled state.
    pub fn new_manual_reset() -> std::io::Result<Self> {
        let handle = unsafe {
            CreateEventW(
                null_mut(), // default security attributes
                1,          // bManualReset = TRUE
                0,          // bInitialState = FALSE (non-signaled)
                null(),     // unnamed
            )
        };
        if handle.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Self { handle })
        }
    }

    /// The raw handle, for passing to `MsgWaitForMultipleObjects`.
    pub fn handle(&self) -> HANDLE {
        self.handle
    }

    /// Transition the event to the signaled state, waking any waiter.
    pub fn set_event(&self) {
        unsafe {
            SetEvent(self.handle);
        }
    }

    /// Transition the event back to the non-signaled state.
    pub fn reset_event(&self) {
        unsafe {
            ResetEvent(self.handle);
        }
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}
