//! Win32 clipboard access for the window backend (CF_UNICODETEXT).
//!
//! Kept intentionally simple: synchronous open/read/close. Callers run on the
//! GUI thread (copy/paste key handling), and `OpenClipboard(NULL)` associates
//! with the calling task, which is sufficient for a terminal.

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock};
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
    CF_UNICODETEXT,
};

const GMEM_MOVEABLE: u32 = 0x0002;

/// Read UTF-16 text from the clipboard, returning an empty string when the
/// clipboard is unavailable or holds no text.
pub(crate) fn get_clipboard_text() -> String {
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return String::new();
        }
        let mut text = String::new();
        let handle = GetClipboardData(CF_UNICODETEXT);
        if !handle.is_null() {
            let ptr = GlobalLock(handle) as *const u16;
            if !ptr.is_null() {
                let mut len = 0usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr, len);
                text = String::from_utf16_lossy(slice);
                GlobalUnlock(handle);
            }
        }
        CloseClipboard();
        text
    }
}

/// Replace the clipboard contents with `text` as CF_UNICODETEXT.
pub(crate) fn set_clipboard_text(text: &str) {
    let wide: Vec<u16> = OsStr::new(text).encode_wide().chain(once(0)).collect();
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return;
        }
        EmptyClipboard();
        let bytes = wide.len() * std::mem::size_of::<u16>();
        let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if !hmem.is_null() {
            let dst = GlobalLock(hmem) as *mut u16;
            if !dst.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
                GlobalUnlock(hmem);
                // On success the system takes ownership of `hmem`; on failure we
                // leak it (rare, negligible) rather than risk a double free.
                SetClipboardData(CF_UNICODETEXT, hmem);
            }
        }
        CloseClipboard();
    }
}
