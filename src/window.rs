//! The settings window: fades and animations, theme and DWM chrome, control activation,
//! hit-testing, and the window proc that handles every message.

use super::*;

pub(crate) unsafe fn invalidate(st: &UiState) {
    if st.hwnd != 0 {
        InvalidateRect(st.hwnd, ptr::null(), 0);
    }
}

pub(crate) unsafe fn ensure_anim_timer(st: &mut UiState) {
    if st.hwnd != 0 && !st.anim_timer_on {
        SetTimer(st.hwnd, TIMER_ANIM, 15, ptr::null());
        st.anim_timer_on = true;
    }
}

pub(crate) unsafe fn start_recording(st: &mut UiState) {
    st.recording = true;
    st.rec_peak = 0;
    st.rec_ticks = 0;
    st.warning = None;
    KillTimer(st.hwnd, TIMER_WARN);
    SetTimer(st.hwnd, TIMER_REC, 30, ptr::null());
    ensure_anim_timer(st);
    invalidate(st);
}

pub(crate) unsafe fn stop_recording(st: &mut UiState) {
    st.recording = false;
    KillTimer(st.hwnd, TIMER_REC);
    ensure_anim_timer(st);
    invalidate(st);
}

pub(crate) unsafe fn show_warning(st: &mut UiState, msg: &str) {
    st.warning = Some(msg.to_string());
    SetTimer(st.hwnd, TIMER_WARN, 4200, ptr::null());
    invalidate(st);
}

pub(crate) unsafe fn show_settings() {
    let (hwnd, was_vis) = with_state(|st| {
        let was_vis = IsWindowVisible(st.hwnd) != 0 && !st.fading_out;
        st.fading_out = false;
        st.warning = None;
        st.keyboard_focus = None;
        KillTimer(st.hwnd, TIMER_WARN);
        st.startup_on = startup_enabled();

        if !was_vis && st.motion {
            st.action_cur = st.action as f32;
            st.preset_cur = st.preset() as f32;
            st.follow_k = if st.follow_mouse { 1.0 } else { 0.0 };
            st.startup_k = if st.startup_on { 1.0 } else { 0.0 };
            st.opacity = 0.0;
            st.fading_in = true;
            SetLayeredWindowAttributes(st.hwnd, 0, 0, LWA_ALPHA);
            ensure_anim_timer(st);
        } else {
            st.opacity = 1.0;
            st.fading_in = false;
            SetLayeredWindowAttributes(st.hwnd, 0, 255, LWA_ALPHA);
        }
        invalidate(st);
        (st.hwnd, was_vis)
    });
    ShowWindow(hwnd, if was_vis { SW_RESTORE } else { SW_SHOWNORMAL });
    SetForegroundWindow(hwnd);
}

pub(crate) unsafe fn fade_out(st: &mut UiState) {
    if IsWindowVisible(st.hwnd) == 0 || st.fading_out {
        return;
    }
    if st.recording {
        stop_recording(st);
    }
    st.fading_in = false;
    st.fading_out = true;
    ensure_anim_timer(st);
}

pub(crate) unsafe fn reset_all(st: &mut UiState) {
    if st.recording {
        stop_recording(st);
    }
    st.warning = None;
    KillTimer(st.hwnd, TIMER_WARN);
    st.action = 0;
    st.use_custom = false;
    st.custom_mods = 0;
    st.follow_mouse = true;
    st.pinch_apps.clear();
    save_pinch_apps(&st.pinch_apps);
    if let Some(touchpad) = &mut st.touchpad {
        touchpad.cancel();
        KillTimer(st.hwnd, TIMER_PINCH);
    }
    save_settings(st);
    set_startup(true);
    st.startup_on = true;
    st.resetting = true;
    sync_zoom_timer(st);
    ensure_anim_timer(st);
    invalidate(st);
}

pub(crate) unsafe fn step_animations(st: &mut UiState) -> Option<HWND> {
    let mut keep = false;
    let mut hide_hwnd = None;

    if st.fading_in {
        st.opacity = if st.motion { (st.opacity + 0.14).min(1.0) } else { 1.0 };
        SetLayeredWindowAttributes(st.hwnd, 0, (st.opacity * 255.0).round() as u8, LWA_ALPHA);
        if st.opacity >= 0.995 {
            st.opacity = 1.0;
            st.fading_in = false;
        } else {
            keep = true;
        }
    } else if st.fading_out {
        st.opacity = if st.motion { (st.opacity - 0.16).max(0.0) } else { 0.0 };
        SetLayeredWindowAttributes(st.hwnd, 0, (st.opacity * 255.0).round() as u8, LWA_ALPHA);
        if st.opacity <= 0.01 {
            st.fading_out = false;
            st.opacity = 1.0;
            SetLayeredWindowAttributes(st.hwnd, 0, 255, LWA_ALPHA);
            st.signal = None;
            KillTimer(st.hwnd, TIMER_SIGNAL);
            hide_hwnd = Some(st.hwnd);
        } else {
            keep = true;
        }
    }

    let motion = st.motion;
    let step_f = |cur: &mut f32, target: f32, speed: f32, active: &mut bool| {
        if (*cur - target).abs() > 0.006 {
            *cur += (target - *cur) * if motion { speed } else { 1.0 };
            if (*cur - target).abs() <= 0.006 {
                *cur = target;
            } else {
                *active = true;
            }
        }
    };

    let preset_target = st.preset() as f32;
    step_f(&mut st.action_cur, st.action as f32, 0.28, &mut keep);
    step_f(&mut st.preset_cur, preset_target, 0.28, &mut keep);
    step_f(&mut st.follow_k, if st.follow_mouse { 1.0 } else { 0.0 }, 0.30, &mut keep);
    step_f(&mut st.startup_k, if st.startup_on { 1.0 } else { 0.0 }, 0.30, &mut keep);
    step_f(&mut st.done_hv, if st.hover_done { 1.0 } else { 0.0 }, 0.30, &mut keep);

    if !keep && st.anim_timer_on {
        KillTimer(st.hwnd, TIMER_ANIM);
        st.anim_timer_on = false;
    }
    invalidate(st);
    hide_hwnd
}

pub(crate) unsafe fn system_dark() -> bool {
    reg_read_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme")
        == Some(0)
}

pub(crate) unsafe fn motion_enabled() -> bool {
    let mut enabled: BOOL = 1;
    SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut enabled as *mut BOOL).cast(), 0);
    enabled != 0
}

pub(crate) unsafe fn apply_dwm_chrome(hwnd: HWND, dark: bool) {
    let palette = Palette::new(dark);
    for (attribute, value) in [
        (DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark)),
        (DWMWA_CAPTION_COLOR, colorref(palette.bg)),
        (DWMWA_TEXT_COLOR, colorref(palette.ink)),
        (DWMWA_BORDER_COLOR, colorref(palette.line)),
    ] {
        DwmSetWindowAttribute(hwnd, attribute, (&value as *const DWORD).cast(), 4);
    }
}

pub(crate) unsafe fn show_signal(st: &mut UiState, control: Control) {
    if st.motion {
        st.signal = Some((control, Instant::now()));
        SetTimer(st.hwnd, TIMER_SIGNAL, 42, ptr::null());
        invalidate(st);
    }
}

pub(crate) unsafe fn activate(st: &mut UiState, control: Control) {
    match control {
        Control::Shortcut => {
            if st.recording { stop_recording(st); } else { start_recording(st); }
        }
        Control::Action(action) => {
            if action != st.action {
                st.action = action;
                st.warning = None;
                KillTimer(st.hwnd, TIMER_WARN);
                save_settings(st);
            }
        }
        Control::Preset(preset) => {
            st.warning = None;
            KillTimer(st.hwnd, TIMER_WARN);
            if preset == 1 && st.custom_mods == 0 {
                if !st.recording {
                    start_recording(st);
                }
            } else {
                if st.recording {
                    stop_recording(st);
                }
                let custom = preset == 1;
                if st.use_custom != custom {
                    st.use_custom = custom;
                    save_settings(st);
                }
            }
        }
        Control::Follow => {
            st.follow_mouse = !st.follow_mouse;
            save_settings(st);
            sync_zoom_timer(st);
        }
        Control::Startup => {
            st.startup_on = !st.startup_on;
            set_startup(st.startup_on);
        }
        Control::Reset => reset_all(st),
        Control::Done => {
            fade_out(st);
            return;
        }
    }
    show_signal(st, control);
    ensure_anim_timer(st);
    invalidate(st);
}

pub(crate) fn seg_hit(r: RectF, s: f32, x: f32, y: f32) -> i32 {
    if !rect_contains(r, x, y) {
        return -1;
    }
    let pad = 4.0 * s;
    let sw = (r.w - pad * 2.0) * 0.5;
    if x < r.x + pad + sw { 0 } else { 1 }
}

pub(crate) fn any_hover(st: &UiState) -> bool {
    st.hover_stage
        || st.hover_action >= 0
        || st.hover_preset >= 0
        || st.hover_follow
        || st.hover_startup
        || st.hover_reset
        || st.hover_done
}

pub(crate) unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc);
            with_state(|st| {
                paint_window(hdc, rc.right - rc.left, rc.bottom - rc.top, st);
            });
            EndPaint(hwnd, &ps);
            0
        }
        WM_SETCURSOR => {
            if (l_param & 0xFFFF) == HTCLIENT {
                let cur = with_state(|st| if any_hover(st) { st.cursor_hand } else { st.cursor_arrow });
                SetCursor(cur);
                return 1;
            }
            DefWindowProcW(hwnd, msg, w_param, l_param)
        }
        WM_MOUSEMOVE => {
            let x = (l_param & 0xFFFF) as i16 as f32;
            let y = ((l_param >> 16) & 0xFFFF) as i16 as f32;
            with_state(|st| {
                if !st.tracking_mouse {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                    st.tracking_mouse = true;
                }
                let lay = Layout::new(st.scale);
                let ns = rect_contains(lay.key_stage, x, y);
                let na = seg_hit(lay.seg_action, lay.s, x, y);
                let np = seg_hit(lay.seg_preset, lay.s, x, y);
                let nf = rect_contains(lay.follow_row, x, y);
                let nu = rect_contains(lay.startup_row, x, y);
                let nr = rect_contains(lay.reset_link, x, y);
                let nd = rect_contains(lay.done_btn, x, y);
                let changed = ns != st.hover_stage
                    || na != st.hover_action
                    || np != st.hover_preset
                    || nf != st.hover_follow
                    || nu != st.hover_startup
                    || nr != st.hover_reset
                    || nd != st.hover_done;
                if nd != st.hover_done {
                    st.hover_done = nd;
                    ensure_anim_timer(st);
                }
                st.hover_stage = ns;
                st.hover_action = na;
                st.hover_preset = np;
                st.hover_follow = nf;
                st.hover_startup = nu;
                st.hover_reset = nr;
                if changed {
                    SetCursor(if any_hover(st) { st.cursor_hand } else { st.cursor_arrow });
                    invalidate(st);
                }
            });
            0
        }
        WM_MOUSELEAVE => {
            with_state(|st| {
                st.tracking_mouse = false;
                let had_done = st.hover_done;
                st.hover_stage = false;
                st.hover_action = -1;
                st.hover_preset = -1;
                st.hover_follow = false;
                st.hover_startup = false;
                st.hover_reset = false;
                st.hover_done = false;
                st.press_done = false;
                if had_done {
                    ensure_anim_timer(st);
                }
                invalidate(st);
            });
            0
        }
        WM_LBUTTONDOWN => {
            let x = (l_param & 0xFFFF) as i16 as f32;
            let y = ((l_param >> 16) & 0xFFFF) as i16 as f32;
            with_state(|st| {
                let lay = Layout::new(st.scale);
                if st.keyboard_focus.take().is_some() {
                    invalidate(st);
                }
                let a = seg_hit(lay.seg_action, lay.s, x, y);
                let p = seg_hit(lay.seg_preset, lay.s, x, y);
                let control = if a >= 0 {
                    Some(Control::Action(a))
                } else if p >= 0 {
                    Some(Control::Preset(p))
                } else {
                    Control::ORDER.into_iter().find(|&control| {
                        !matches!(control, Control::Action(_) | Control::Preset(_))
                            && rect_contains(lay.control_rect(control), x, y)
                    })
                };
                match control {
                    Some(Control::Done) => {
                        st.press_done = true;
                        show_signal(st, Control::Done);
                        invalidate(st);
                    }
                    Some(control) => activate(st, control),
                    None => {}
                }
            });
            0
        }
        WM_LBUTTONUP => {
            let x = (l_param & 0xFFFF) as i16 as f32;
            let y = ((l_param >> 16) & 0xFFFF) as i16 as f32;
            with_state(|st| {
                if st.press_done {
                    st.press_done = false;
                    let lay = Layout::new(st.scale);
                    if rect_contains(lay.done_btn, x, y) {
                        fade_out(st);
                    } else {
                        invalidate(st);
                    }
                }
            });
            0
        }
        WM_KEYDOWN => {
            let vk = w_param as i32;
            with_state(|st| {
                if vk == VK_ESCAPE {
                    if st.recording { stop_recording(st); } else { fade_out(st); }
                } else if !st.recording && vk == VK_TAB {
                    let order = Control::ORDER;
                    let previous = st.keyboard_focus.and_then(|focus| order.iter().position(|c| *c == focus));
                    let reverse = key_down(VK_SHIFT);
                    let next = match previous {
                        Some(index) if reverse => (index + order.len() - 1) % order.len(),
                        Some(index) => (index + 1) % order.len(),
                        None if reverse => order.len() - 1,
                        None => 0,
                    };
                    st.keyboard_focus = Some(order[next]);
                    invalidate(st);
                } else if !st.recording && (vk == VK_RETURN || vk == VK_SPACE) && l_param & (1 << 30) == 0 {
                    if let Some(control) = st.keyboard_focus {
                        activate(st, control);
                    } else if vk == VK_RETURN {
                        fade_out(st);
                    }
                }
            });
            0
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED => {
            let dark = system_dark();
            let changed = with_state(|st| {
                let changed = st.dark != dark;
                st.dark = dark;
                st.motion = motion_enabled();
                if !st.motion {
                    st.signal = None;
                    KillTimer(hwnd, TIMER_SIGNAL);
                }
                invalidate(st);
                changed
            });
            if changed {
                apply_dwm_chrome(hwnd, dark);
            }
            0
        }
        WM_TIMER => {
            if w_param == TIMER_PINCH {
                let window = touchpad::cursor_window();
                with_state(|st| {
                    if let Some(touchpad) = &mut st.touchpad {
                        touchpad.tick(window);
                        if touchpad.active() {
                            return;
                        }
                    }
                    KillTimer(hwnd, TIMER_PINCH);
                });
                return 0;
            }
            let mut hide_hwnd = None;
            let mut zoom_tf = None;
            with_state(|st| match w_param {
                TIMER_ZOOM => {
                    if st.resetting {
                        st.zoom = 1.0 + (st.zoom - 1.0) * 0.88;
                        if st.zoom < 1.005 {
                            st.zoom = 1.0;
                            st.resetting = false;
                        }
                        zoom_tf = compute_zoom_transform(st);
                        if !st.resetting {
                            sync_zoom_timer(st);
                        }
                    } else if st.zoom > 1.0 && st.follow_mouse {
                        zoom_tf = compute_zoom_transform(st);
                    } else {
                        sync_zoom_timer(st);
                    }
                }
                TIMER_ANIM => hide_hwnd = step_animations(st),
                TIMER_SIGNAL => {
                    if st.signal.is_some_and(|(_, start)| start.elapsed().as_millis() >= 530) {
                        st.signal = None;
                        KillTimer(hwnd, TIMER_SIGNAL);
                    }
                    invalidate(st);
                }
                TIMER_REC => {
                    let previous = st.rec_peak;
                    let wave_was_visible = st.motion && draw::recording_wave(st.rec_ticks).is_some();
                    let m = current_mods();
                    st.rec_ticks += 1;
                    if m != 0 {
                        st.rec_peak |= m;
                    }
                    if st.rec_peak != 0 && m == 0 {
                        let got = st.rec_peak;
                        stop_recording(st);
                        if got == MOD_SHIFT {
                            show_warning(st, "Shift alone clashes with horizontal scroll. Add another key.");
                            return;
                        }
                        st.custom_mods = got;
                        st.use_custom = true;
                        save_settings(st);
                        ensure_anim_timer(st);
                        invalidate(st);
                        return;
                    }
                    let wave_is_visible = st.motion && draw::recording_wave(st.rec_ticks).is_some();
                    if st.rec_ticks > 300 {
                        stop_recording(st);
                    } else if previous != st.rec_peak || wave_was_visible || wave_is_visible {
                        invalidate(st);
                    }
                }
                TIMER_WARN => {
                    st.warning = None;
                    KillTimer(hwnd, TIMER_WARN);
                    invalidate(st);
                }
                _ => {}
            });
            if zoom_tf.is_some() {
                apply_transform(zoom_tf);
            }
            if let Some(h) = hide_hwnd {
                ShowWindow(h, SW_HIDE);
            }
            0
        }
        WM_APP_TRAY => {
            let ev = (l_param & 0xFFFF) as UINT;
            if ev == WM_LBUTTONDBLCLK {
                show_settings();
            } else if ev == WM_RBUTTONUP {
                show_tray_menu();
            }
            0
        }
        WM_APP_SHOW => {
            show_settings();
            0
        }
        WM_CLOSE => {
            with_state(|st| fade_out(st));
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => {
            let tc = try_with_state(|st| (st.taskbar_created_msg, st.hicon));
            if let Some((tc_msg, hicon)) = tc {
                if tc_msg != 0 && msg == tc_msg {
                    add_tray_icon(hwnd, hicon);
                    return 0;
                }
            }
            DefWindowProcW(hwnd, msg, w_param, l_param)
        }
    }
}
