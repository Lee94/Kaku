//! Win32 virtual-key / modifier / mouse-button translation for the window
//! backend. Text-producing keys are handled via `WM_CHAR` (which already applies
//! the keyboard layout and produces control chars for Ctrl combos); this module
//! covers the navigation/function keys that do not generate `WM_CHAR`, plus the
//! current modifier and mouse-button state.

use crate::{KeyCode, Modifiers, MouseButtons};
use winapi::um::winuser::*;

/// Snapshot the currently-held modifier keys.
pub(crate) fn current_modifiers() -> Modifiers {
    let mut mods = Modifiers::NONE;
    unsafe {
        if GetKeyState(VK_SHIFT) < 0 {
            mods |= Modifiers::SHIFT;
        }
        if GetKeyState(VK_CONTROL) < 0 {
            mods |= Modifiers::CTRL;
        }
        if GetKeyState(VK_MENU) < 0 {
            mods |= Modifiers::ALT;
        }
        if GetKeyState(VK_LWIN) < 0 || GetKeyState(VK_RWIN) < 0 {
            mods |= Modifiers::SUPER;
        }
    }
    mods
}

/// Map a Win32 virtual key to a `KeyCode` for keys that do NOT produce a
/// `WM_CHAR` message (arrows, navigation, function keys, Delete). Returns `None`
/// for keys whose textual value is delivered via `WM_CHAR` instead.
pub(crate) fn vkey_to_keycode(vk: i32) -> Option<KeyCode> {
    let kc = match vk {
        VK_LEFT => KeyCode::LeftArrow,
        VK_RIGHT => KeyCode::RightArrow,
        VK_UP => KeyCode::UpArrow,
        VK_DOWN => KeyCode::DownArrow,
        VK_HOME => KeyCode::Home,
        VK_END => KeyCode::End,
        VK_PRIOR => KeyCode::PageUp,
        VK_NEXT => KeyCode::PageDown,
        VK_INSERT => KeyCode::Insert,
        // Delete does not emit WM_CHAR; deliver it as DEL.
        VK_DELETE => KeyCode::Char('\u{7f}'),
        _ if vk >= VK_F1 && vk <= VK_F24 => KeyCode::Function((vk - VK_F1 + 1) as u8),
        _ => return None,
    };
    Some(kc)
}

/// Build the pressed mouse-button set from a message's `wparam` MK_* flags.
pub(crate) fn mouse_buttons_from_wparam(wparam: usize) -> MouseButtons {
    let mut b = MouseButtons::NONE;
    if wparam & (MK_LBUTTON as usize) != 0 {
        b |= MouseButtons::LEFT;
    }
    if wparam & (MK_RBUTTON as usize) != 0 {
        b |= MouseButtons::RIGHT;
    }
    if wparam & (MK_MBUTTON as usize) != 0 {
        b |= MouseButtons::MIDDLE;
    }
    b
}
