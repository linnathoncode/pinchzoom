//! The zoom engine: magnification transform, zoom/reset timer, pinch queueing,
//! and the low-level mouse and keyboard hooks that drive them.

use super::*;

pub(crate) unsafe fn compute_zoom_transform(st: &mut UiState) -> Option<(f32, i32, i32)> {
    if st.zoom <= 1.0001 {
        st.has_anchor = false;
        return Some((1.0, 0, 0));
    }
    let (fx, fy) = if st.follow_mouse || !st.has_anchor {
        let mut p = POINT::default();
        if GetCursorPos(&mut p) == 0 {
            return None;
        }
        st.anchor_x = p.x;
        st.anchor_y = p.y;
        st.has_anchor = true;
        (p.x, p.y)
    } else {
        (st.anchor_x, st.anchor_y)
    };
    let k = 1.0 - 1.0 / st.zoom;
    Some((st.zoom, (fx as f32 * k).round() as i32, (fy as f32 * k).round() as i32))
}

pub(crate) unsafe fn apply_transform(tf: Option<(f32, i32, i32)>) {
    if let Some((z, ox, oy)) = tf {
        MagSetFullscreenTransform(z, ox, oy);
    }
}

pub(crate) unsafe fn sync_zoom_timer(st: &mut UiState) {
    if st.hwnd == 0 {
        return;
    }
    let need = st.resetting || (st.zoom > 1.0 && st.follow_mouse);
    if need && !st.zoom_timer_on {
        SetTimer(st.hwnd, TIMER_ZOOM, 10, ptr::null());
        st.zoom_timer_on = true;
    } else if !need && st.zoom_timer_on {
        KillTimer(st.hwnd, TIMER_ZOOM);
        st.zoom_timer_on = false;
    }
}

pub(crate) unsafe fn queue_pinch(st: &mut UiState, point: Option<POINT>, delta: i32) -> bool {
    if st.zoom > 1.0 || st.pinch_apps.is_empty() || st.touchpad.is_none() {
        return false;
    }
    let window = point.map_or_else(touchpad::cursor_window, touchpad::window_at);
    let Some(path) = touchpad::app_path(window) else {
        return false;
    };
    if !st.pinch_apps.contains(&path) {
        return false;
    }
    let touchpad = st.touchpad.as_mut().unwrap();
    if !touchpad.ready() {
        return false;
    }
    let was_active = touchpad.active();
    touchpad.queue(window, delta);
    if !was_active && touchpad.active()
        && SetTimer(st.hwnd, TIMER_PINCH, touchpad::FRAME_MS, ptr::null()) == 0
    {
        touchpad.cancel();
        return false;
    }
    true
}

pub(crate) const MAX_ZOOM: f32 = 10.0;
/** Zoom factor for one numpad +/- press. */
pub(crate) const KEY_ZOOM_STEP: f32 = 1.05;

/// Zooms in or out by `step`, clamped to 1x..MAX_ZOOM, and cancels a reset in progress.
pub(crate) fn zoom_by(st: &mut UiState, zoom_in: bool, step: f32) {
    st.resetting = false;
    st.zoom = if zoom_in { (st.zoom * step).min(MAX_ZOOM) } else { (st.zoom / step).max(1.0) };
}

/// What a hook decided while it held the UI state. The side effects run after the
/// state borrow is released, because they can re-enter the window proc.
#[derive(Default)]
pub(crate) struct HookOutcome {
    handled: bool,
    next_hook: HHOOK,
    tf: Option<(f32, i32, i32)>,
    mask_mods: i32,
}

pub(crate) unsafe fn finish_hook(out: HookOutcome, n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if out.tf.is_some() {
        apply_transform(out.tf);
    }
    if out.mask_mods != 0 {
        mask_key(out.mask_mods);
    }
    if out.handled { 1 } else { CallNextHookEx(out.next_hook, n_code, w_param, l_param) }
}

pub(crate) unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let mut out = HookOutcome::default();
    try_with_state(|st| {
        out.next_hook = st.mouse_hook;
        let msg = w_param as UINT;
        let ours = msg == WM_MOUSEWHEEL || msg == WM_MBUTTONDOWN || msg == WM_MBUTTONUP;
        // Cheap checks first: this runs for every mouse move.
        if n_code < 0 || st.action != 0 || !ours || current_mods() != st.hot_mods {
            return;
        }
        if msg == WM_MOUSEWHEEL {
            let s = &*(l_param as *const MSLLHOOKSTRUCT);
            let delta = ((s.mouseData >> 16) & 0xFFFF) as i16;
            if s.flags & 1 != 0 && st.touchpad.as_ref().is_some_and(touchpad::Touchpad::active) {
                return;
            }
            if !queue_pinch(st, Some(s.pt), i32::from(delta)) {
                let step = 1.1_f64.powf((delta as f64).abs() / 120.0) as f32;
                zoom_by(st, delta > 0, step);
                out.tf = compute_zoom_transform(st);
                sync_zoom_timer(st);
            }
            out.mask_mods = st.hot_mods;
            out.handled = true;
        } else if (msg == WM_MBUTTONDOWN || msg == WM_MBUTTONUP) && st.zoom > 1.0 {
            if msg == WM_MBUTTONDOWN {
                st.resetting = true;
                sync_zoom_timer(st);
                out.mask_mods = st.hot_mods;
            }
            out.handled = true;
        }
    });
    finish_hook(out, n_code, w_param, l_param)
}

pub(crate) unsafe extern "system" fn kb_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let mut out = HookOutcome::default();
    try_with_state(|st| {
        out.next_hook = st.kb_hook;
        if n_code < 0 || st.action != 1 {
            return;
        }
        let k = &*(l_param as *const KBDLLHOOKSTRUCT);
        let msg = w_param as UINT;
        let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
        let ours = k.vkCode == VK_ADD || k.vkCode == VK_SUBTRACT || k.vkCode == VK_MULTIPLY;
        if !ours || !(is_down || is_up) || current_mods() != st.hot_mods {
            return;
        }
        out.handled = true;
        if !is_down {
            return;
        }
        out.mask_mods = st.hot_mods;
        let delta = if k.vkCode == VK_ADD { touchpad::NUMPAD_STEP } else { -touchpad::NUMPAD_STEP };
        if k.vkCode != VK_MULTIPLY && queue_pinch(st, None, delta) {
            return;
        }
        if k.vkCode == VK_MULTIPLY {
            st.resetting = true;
        } else {
            zoom_by(st, k.vkCode == VK_ADD, KEY_ZOOM_STEP);
        }
        out.tf = compute_zoom_transform(st);
        sync_zoom_timer(st);
    });
    finish_hook(out, n_code, w_param, l_param)
}
