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

/// Map a virtual key to its base (unshifted, lowercase) character, for use when
/// Ctrl/Alt is held so keybindings receive `Char(c)` + modifiers instead of a
/// cooked control character from WM_CHAR. Returns None for non-character keys.
pub(crate) fn vkey_to_char(vk: i32) -> Option<char> {
    match vk {
        // A-Z -> lowercase so the SHIFT modifier stays explicit.
        0x41..=0x5A => Some((b'a' + (vk - 0x41) as u8) as char),
        0x30..=0x39 => Some((b'0' + (vk - 0x30) as u8) as char),
        _ => {
            // Punctuation/symbols: ask the active layout for the unshifted char.
            let ch = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_CHAR) } & 0x7fff;
            if ch != 0 {
                char::from_u32(ch).filter(|c| !c.is_control())
            } else {
                None
            }
        }
    }
}

/// Map a `KeyCode` to a Win32 virtual key, for RegisterHotKey. Covers the keys a
/// global hotkey would realistically use (letters, digits, function, navigation).
pub(crate) fn keycode_to_vk(key: &KeyCode) -> Option<u32> {
    Some(match key {
        KeyCode::Char(c) => {
            let u = c.to_ascii_uppercase();
            if u.is_ascii_alphanumeric() {
                u as u32
            } else {
                return None;
            }
        }
        KeyCode::Function(n) if *n >= 1 && *n <= 24 => VK_F1 as u32 + (*n as u32 - 1),
        KeyCode::LeftArrow => VK_LEFT as u32,
        KeyCode::RightArrow => VK_RIGHT as u32,
        KeyCode::UpArrow => VK_UP as u32,
        KeyCode::DownArrow => VK_DOWN as u32,
        KeyCode::Home => VK_HOME as u32,
        KeyCode::End => VK_END as u32,
        KeyCode::PageUp => VK_PRIOR as u32,
        KeyCode::PageDown => VK_NEXT as u32,
        KeyCode::Insert => VK_INSERT as u32,
        _ => return None,
    })
}

/// Translate our modifiers to the RegisterHotKey MOD_* flags.
pub(crate) fn mods_to_win32(mods: Modifiers) -> u32 {
    let mut m = 0u32;
    if mods.contains(Modifiers::ALT) {
        m |= MOD_ALT as u32;
    }
    if mods.contains(Modifiers::CTRL) {
        m |= MOD_CONTROL as u32;
    }
    if mods.contains(Modifiers::SHIFT) {
        m |= MOD_SHIFT as u32;
    }
    if mods.contains(Modifiers::SUPER) {
        m |= MOD_WIN as u32;
    }
    m
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
