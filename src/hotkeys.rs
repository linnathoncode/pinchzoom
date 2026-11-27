//! The modifier-key combination that arms zooming: reading it from the keyboard,
//! masking it so the target app never sees it, and describing it in labels.

use super::*;

pub const MOD_CTRL: i32 = 1;
pub const MOD_ALT: i32 = 2;
pub const MOD_SHIFT: i32 = 4;
pub const MOD_WIN: i32 = 8;
pub const DEFAULT_MODS: i32 = MOD_CTRL | MOD_ALT;

pub fn format_mods(mods: i32) -> String {
    let mut parts = Vec::new();
    if (mods & MOD_CTRL) != 0 { parts.push("Ctrl"); }
    if (mods & MOD_ALT) != 0 { parts.push("Alt"); }
    if (mods & MOD_SHIFT) != 0 { parts.push("Shift"); }
    if (mods & MOD_WIN) != 0 { parts.push("Win"); }
    parts.join(" + ")
}

pub fn format_zoom_summary(mods: i32, act: i32) -> String {
    let m = format_mods(mods);
    let suffix = if act == 0 { "Scroll wheel" } else { "Num + / \u{2212}" };
    if m.is_empty() { suffix.to_string() } else { format!("{} + {}", m, suffix) }
}

pub fn format_reset_summary(mods: i32, act: i32) -> String {
    let m = format_mods(mods);
    let suffix = if act == 0 { "Middle click" } else { "Num *" };
    if m.is_empty() { suffix.to_string() } else { format!("{} + {}", m, suffix) }
}

pub(crate) unsafe fn key_down(vk: i32) -> bool {
    (GetAsyncKeyState(vk) & -0x8000i16) != 0
}

pub(crate) unsafe fn current_mods() -> i32 {
    let mut m = 0;
    if key_down(VK_CONTROL) { m |= MOD_CTRL; }
    if key_down(VK_MENU) { m |= MOD_ALT; }
    if key_down(VK_SHIFT) { m |= MOD_SHIFT; }
    if key_down(VK_LWIN) || key_down(VK_RWIN) { m |= MOD_WIN; }
    m
}

pub(crate) unsafe fn mask_key(hot_mods: i32) {
    if (hot_mods & (MOD_WIN | MOD_ALT)) == 0 {
        return;
    }
    keybd_event(0xE8, 0, 0, 0);
    keybd_event(0xE8, 0, 2, 0);
}

// Computes the magnification transform without calling MagSetFullscreenTransform while STATE is borrowed.
