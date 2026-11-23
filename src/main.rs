#![windows_subsystem = "windows"]

mod draw;
mod win32;

use std::cell::RefCell;
use std::ptr;
use std::time::Instant;

use draw::{paint_window, rect_contains, Control, Layout, Palette, CLIENT_H, CLIENT_W};
use win32::*;

pub const MOD_CTRL: i32 = 1;
pub const MOD_ALT: i32 = 2;
pub const MOD_SHIFT: i32 = 4;
pub const MOD_WIN: i32 = 8;
pub const DEFAULT_MODS: i32 = MOD_CTRL | MOD_ALT;

const APP_NAME: &str = "pinchzoom";
const REG_NAME: &str = "PinchZoom";
const SETTINGS_KEY: &str = "Software\\PinchZoom";
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_APPROVED_KEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
const WND_CLASS: &str = "PinchZoomSettingsWnd";

const WM_APP_TRAY: UINT = WM_APP + 1;
const WM_APP_SHOW: UINT = WM_APP + 2;

const TIMER_ZOOM: usize = 1;
const TIMER_ANIM: usize = 2;
const TIMER_REC: usize = 3;
const TIMER_WARN: usize = 4;
const TIMER_SIGNAL: usize = 5;

const MENU_SETTINGS: usize = 1001;
const MENU_RESET: usize = 1002;
const MENU_QUIT: usize = 1003;

pub struct UiState {
    pub hwnd: HWND,
    pub scale: f32,
    pub dark: bool,
    pub motion: bool,
    pub hicon: HICON,
    pub cursor_arrow: HCURSOR,
    pub cursor_hand: HCURSOR,

    // Zoom engine state
    pub zoom: f32,
    pub resetting: bool,
    pub hot_mods: i32,
    pub custom_mods: i32,
    pub use_custom: bool,
    pub action: i32,
    pub follow_mouse: bool,
    pub startup_on: bool,
    pub anchor_x: i32,
    pub anchor_y: i32,
    pub has_anchor: bool,
    pub zoom_timer_on: bool,

    // Recording & warning state
    pub recording: bool,
    pub rec_peak: i32,
    pub rec_ticks: i32,
    pub warning: Option<String>,

    // Animation state
    pub action_cur: f32,
    pub preset_cur: f32,
    pub follow_k: f32,
    pub startup_k: f32,
    pub done_hv: f32,
    pub signal: Option<(Control, Instant)>,
    pub opacity: f32,
    pub fading_in: bool,
    pub fading_out: bool,
    pub anim_timer_on: bool,

    // Hover & press state
    pub keyboard_focus: Option<Control>,
    pub tracking_mouse: bool,
    pub hover_stage: bool,
    pub hover_action: i32,
    pub hover_preset: i32,
    pub hover_follow: bool,
    pub hover_startup: bool,
    pub hover_reset: bool,
    pub hover_done: bool,
    pub press_done: bool,

    // Hooks & tray recovery
    pub mouse_hook: HHOOK,
    pub kb_hook: HHOOK,
    pub taskbar_created_msg: UINT,
}

impl UiState {
    fn new(scale: f32) -> Self {
        Self {
            hwnd: 0,
            scale,
            dark: false,
            motion: true,
            hicon: 0,
            cursor_arrow: 0,
            cursor_hand: 0,
            zoom: 1.0,
            resetting: false,
            hot_mods: DEFAULT_MODS,
            custom_mods: 0,
            use_custom: false,
            action: 0,
            follow_mouse: true,
            startup_on: false,
            anchor_x: 0,
            anchor_y: 0,
            has_anchor: false,
            zoom_timer_on: false,
            recording: false,
            rec_peak: 0,
            rec_ticks: 0,
            warning: None,
            action_cur: 0.0,
            preset_cur: 0.0,
            follow_k: 1.0,
            startup_k: 0.0,
            done_hv: 0.0,
            signal: None,
            opacity: 1.0,
            fading_in: false,
            fading_out: false,
            anim_timer_on: false,
            keyboard_focus: None,
            tracking_mouse: false,
            hover_stage: false,
            hover_action: -1,
            hover_preset: -1,
            hover_follow: false,
            hover_startup: false,
            hover_reset: false,
            hover_done: false,
            press_done: false,
            mouse_hook: 0,
            kb_hook: 0,
            taskbar_created_msg: 0,
        }
    }

    pub fn active_mods(&self) -> i32 {
        if self.use_custom && self.custom_mods != 0 {
            self.custom_mods
        } else {
            DEFAULT_MODS
        }
    }

    pub fn preset(&self) -> i32 {
        i32::from((self.use_custom && self.custom_mods != 0) || self.recording)
    }

    fn update_active(&mut self) {
        self.hot_mods = self.active_mods();
    }
}

thread_local! {
    static STATE: RefCell<Option<UiState>> = const { RefCell::new(None) };
}

fn try_with_state<R>(f: impl FnOnce(&mut UiState) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut opt = cell.try_borrow_mut().ok()?;
        opt.as_mut().map(f)
    })
}

fn with_state<R>(f: impl FnOnce(&mut UiState) -> R) -> R {
    STATE.with(|cell| {
        let mut opt = cell.borrow_mut();
        f(opt.as_mut().expect("state initialized"))
    })
}

// ---------- formatting ----------

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

// ---------- zoom engine ----------

unsafe fn key_down(vk: i32) -> bool {
    (GetAsyncKeyState(vk) & -0x8000i16) != 0
}

unsafe fn current_mods() -> i32 {
    let mut m = 0;
    if key_down(VK_CONTROL) { m |= MOD_CTRL; }
    if key_down(VK_MENU) { m |= MOD_ALT; }
    if key_down(VK_SHIFT) { m |= MOD_SHIFT; }
    if key_down(VK_LWIN) || key_down(VK_RWIN) { m |= MOD_WIN; }
    m
}

unsafe fn mask_key(hot_mods: i32) {
    if (hot_mods & (MOD_WIN | MOD_ALT)) == 0 {
        return;
    }
    keybd_event(0xE8, 0, 0, 0);
    keybd_event(0xE8, 0, 2, 0);
}

// Computes the magnification transform without calling MagSetFullscreenTransform while STATE is borrowed.
unsafe fn compute_zoom_transform(st: &mut UiState) -> Option<(f32, i32, i32)> {
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

unsafe fn apply_transform(tf: Option<(f32, i32, i32)>) {
    if let Some((z, ox, oy)) = tf {
        MagSetFullscreenTransform(z, ox, oy);
    }
}

unsafe fn sync_zoom_timer(st: &mut UiState) {
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

unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let mut handled = false;
    let mut next_hook = 0;
    let mut tf = None;
    let mut mask_mods = 0;
    try_with_state(|st| {
        next_hook = st.mouse_hook;
        if n_code >= 0 && st.action == 0 {
            let msg = w_param as UINT;
            if msg == WM_MOUSEWHEEL && current_mods() == st.hot_mods {
                let s = &*(l_param as *const MSLLHOOKSTRUCT);
                let delta = ((s.mouseData >> 16) & 0xFFFF) as i16;
                let step = 1.1_f64.powf((delta as f64).abs() / 120.0) as f32;
                st.resetting = false;
                st.zoom = if delta > 0 {
                    (st.zoom * step).min(10.0)
                } else {
                    (st.zoom / step).max(1.0)
                };
                tf = compute_zoom_transform(st);
                sync_zoom_timer(st);
                mask_mods = st.hot_mods;
                handled = true;
            } else if (msg == WM_MBUTTONDOWN || msg == WM_MBUTTONUP)
                && current_mods() == st.hot_mods
                && st.zoom > 1.0
            {
                if msg == WM_MBUTTONDOWN {
                    st.resetting = true;
                    sync_zoom_timer(st);
                    mask_mods = st.hot_mods;
                }
                handled = true;
            }
        }
    });
    if tf.is_some() {
        apply_transform(tf);
    }
    if mask_mods != 0 {
        mask_key(mask_mods);
    }
    if handled { 1 } else { CallNextHookEx(next_hook, n_code, w_param, l_param) }
}

unsafe extern "system" fn kb_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let mut handled = false;
    let mut next_hook = 0;
    let mut tf = None;
    let mut mask_mods = 0;
    try_with_state(|st| {
        next_hook = st.kb_hook;
        if n_code >= 0 && st.action == 1 {
            let k = &*(l_param as *const KBDLLHOOKSTRUCT);
            let msg = w_param as UINT;
            let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            let ours = k.vkCode == VK_ADD || k.vkCode == VK_SUBTRACT || k.vkCode == VK_MULTIPLY;
            if ours && (is_down || is_up) && current_mods() == st.hot_mods {
                if is_down {
                    if k.vkCode == VK_MULTIPLY {
                        st.resetting = true;
                    } else {
                        st.resetting = false;
                        if k.vkCode == VK_ADD {
                            st.zoom = (st.zoom * 1.05).min(10.0);
                        } else {
                            st.zoom = (st.zoom / 1.05).max(1.0);
                        }
                    }
                    tf = compute_zoom_transform(st);
                    sync_zoom_timer(st);
                    mask_mods = st.hot_mods;
                }
                handled = true;
            }
        }
    });
    if tf.is_some() {
        apply_transform(tf);
    }
    if mask_mods != 0 {
        mask_key(mask_mods);
    }
    if handled { 1 } else { CallNextHookEx(next_hook, n_code, w_param, l_param) }
}

// ---------- registry storage ----------

unsafe fn reg_read_dword(subkey: &str, name: &str) -> Option<i32> {
    let mut hkey: HKEY = 0;
    let sk = wide(subkey);
    if RegOpenKeyExW(HKEY_CURRENT_USER, sk.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
        return None;
    }
    let vn = wide(name);
    let mut ty: DWORD = 0;
    let mut val: u32 = 0;
    let mut cb: DWORD = 4;
    let ok = RegQueryValueExW(
        hkey,
        vn.as_ptr(),
        ptr::null(),
        &mut ty,
        (&mut val as *mut u32) as *mut u8,
        &mut cb,
    ) == 0
        && ty == REG_DWORD;
    RegCloseKey(hkey);
    if ok { Some(val as i32) } else { None }
}

unsafe fn reg_write_dword(subkey: &str, name: &str, value: i32) {
    let mut hkey: HKEY = 0;
    let sk = wide(subkey);
    if RegCreateKeyExW(
        HKEY_CURRENT_USER,
        sk.as_ptr(),
        0,
        ptr::null(),
        0,
        KEY_WRITE,
        ptr::null(),
        &mut hkey,
        ptr::null_mut(),
    ) != 0
    {
        return;
    }
    let vn = wide(name);
    let v = value as u32;
    RegSetValueExW(hkey, vn.as_ptr(), 0, REG_DWORD, (&v as *const u32) as *const u8, 4);
    RegCloseKey(hkey);
}

unsafe fn load_settings(st: &mut UiState) {
    if let Some(c) = reg_read_dword(SETTINGS_KEY, "CustomModifiers") {
        st.custom_mods = c;
    }
    if let Some(u) = reg_read_dword(SETTINGS_KEY, "UseCustom") {
        st.use_custom = u != 0;
    }
    if let Some(a) = reg_read_dword(SETTINGS_KEY, "Action") {
        st.action = if a == 1 { 1 } else { 0 };
    }
    if let Some(f) = reg_read_dword(SETTINGS_KEY, "FollowMouse") {
        st.follow_mouse = f != 0;
    }
    st.update_active();
}

unsafe fn save_settings(st: &mut UiState) {
    st.update_active();
    reg_write_dword(SETTINGS_KEY, "CustomModifiers", st.custom_mods);
    reg_write_dword(SETTINGS_KEY, "UseCustom", if st.use_custom { 1 } else { 0 });
    reg_write_dword(SETTINGS_KEY, "Action", st.action);
    reg_write_dword(SETTINGS_KEY, "FollowMouse", if st.follow_mouse { 1 } else { 0 });
}

unsafe fn startup_enabled() -> bool {
    let mut hkey: HKEY = 0;
    let sk = wide(RUN_KEY);
    if RegOpenKeyExW(HKEY_CURRENT_USER, sk.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
        return false;
    }
    let vn = wide(REG_NAME);
    let mut ty: DWORD = 0;
    let mut cb: DWORD = 0;
    let exists = RegQueryValueExW(hkey, vn.as_ptr(), ptr::null(), &mut ty, ptr::null_mut(), &mut cb) == 0;
    RegCloseKey(hkey);
    exists
}

unsafe fn exe_path() -> String {
    let mut buf = [0u16; 1024];
    let len = GetModuleFileNameW(0, buf.as_mut_ptr(), buf.len() as DWORD) as usize;
    String::from_utf16_lossy(&buf[..len])
}

unsafe fn set_startup(on: bool) {
    let mut hkey: HKEY = 0;
    let sk = wide(RUN_KEY);
    if RegOpenKeyExW(HKEY_CURRENT_USER, sk.as_ptr(), 0, KEY_WRITE, &mut hkey) == 0 {
        let vn = wide(REG_NAME);
        if on {
            let cmd = format!("\"{}\" --background", exe_path());
            let wcmd = wide(&cmd);
            RegSetValueExW(
                hkey,
                vn.as_ptr(),
                0,
                REG_SZ,
                wcmd.as_ptr() as *const u8,
                (wcmd.len() * 2) as DWORD,
            );
        } else {
            RegDeleteValueW(hkey, vn.as_ptr());
        }
        RegCloseKey(hkey);
    }

    let mut akey: HKEY = 0;
    let ask = wide(STARTUP_APPROVED_KEY);
    if RegCreateKeyExW(
        HKEY_CURRENT_USER,
        ask.as_ptr(),
        0,
        ptr::null(),
        0,
        KEY_WRITE,
        ptr::null(),
        &mut akey,
        ptr::null_mut(),
    ) == 0
    {
        let vn = wide(REG_NAME);
        if on {
            let enabled: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            RegSetValueExW(akey, vn.as_ptr(), 0, REG_BINARY, enabled.as_ptr(), 12);
        } else {
            RegDeleteValueW(akey, vn.as_ptr());
        }
        RegCloseKey(akey);
    }
}

unsafe fn ensure_startup_default() {
    if reg_read_dword(SETTINGS_KEY, "StartupInit").is_none() {
        set_startup(true);
        reg_write_dword(SETTINGS_KEY, "StartupInit", 1);
    }
}

// ---------- window helpers & animations ----------

unsafe fn invalidate(st: &UiState) {
    if st.hwnd != 0 {
        InvalidateRect(st.hwnd, ptr::null(), 0);
    }
}

unsafe fn ensure_anim_timer(st: &mut UiState) {
    if st.hwnd != 0 && !st.anim_timer_on {
        SetTimer(st.hwnd, TIMER_ANIM, 15, ptr::null());
        st.anim_timer_on = true;
    }
}

unsafe fn start_recording(st: &mut UiState) {
    st.recording = true;
    st.rec_peak = 0;
    st.rec_ticks = 0;
    st.warning = None;
    KillTimer(st.hwnd, TIMER_WARN);
    SetTimer(st.hwnd, TIMER_REC, 30, ptr::null());
    ensure_anim_timer(st);
    invalidate(st);
}

unsafe fn stop_recording(st: &mut UiState) {
    st.recording = false;
    KillTimer(st.hwnd, TIMER_REC);
    ensure_anim_timer(st);
    invalidate(st);
}

unsafe fn show_warning(st: &mut UiState, msg: &str) {
    st.warning = Some(msg.to_string());
    SetTimer(st.hwnd, TIMER_WARN, 4200, ptr::null());
    invalidate(st);
}

unsafe fn show_settings() {
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

unsafe fn fade_out(st: &mut UiState) {
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

unsafe fn reset_all(st: &mut UiState) {
    if st.recording {
        stop_recording(st);
    }
    st.warning = None;
    KillTimer(st.hwnd, TIMER_WARN);
    st.action = 0;
    st.use_custom = false;
    st.custom_mods = 0;
    st.follow_mouse = true;
    save_settings(st);
    set_startup(true);
    st.startup_on = true;
    st.resetting = true;
    sync_zoom_timer(st);
    ensure_anim_timer(st);
    invalidate(st);
}

unsafe fn step_animations(st: &mut UiState) -> Option<HWND> {
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

unsafe fn show_tray_menu() {
    let (hwnd, hot_mods, action) = with_state(|st| (st.hwnd, st.hot_mods, st.action));
    let menu = CreatePopupMenu();
    let summary = wide(&format_zoom_summary(hot_mods, action));
    AppendMenuW(menu, MF_STRING | MF_GRAYED | MF_DISABLED, 0, summary.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    let s_set = wide("Settings");
    let s_rst = wide("Reset zoom");
    let s_quit = wide(&format!("Quit {}", APP_NAME));
    AppendMenuW(menu, MF_STRING, MENU_SETTINGS, s_set.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_RESET, s_rst.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(menu, MF_STRING, MENU_QUIT, s_quit.as_ptr());

    let mut pt = POINT::default();
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenuEx(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD, pt.x, pt.y, hwnd, ptr::null()) as usize;
    DestroyMenu(menu);

    if cmd == MENU_SETTINGS {
        show_settings();
    } else if cmd == MENU_RESET {
        with_state(|st| {
            st.resetting = true;
            sync_zoom_timer(st);
        });
    } else if cmd == MENU_QUIT {
        PostQuitMessage(0);
    }
}

unsafe fn system_dark() -> bool {
    reg_read_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme")
        == Some(0)
}

unsafe fn motion_enabled() -> bool {
    let mut enabled: BOOL = 1;
    SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&mut enabled as *mut BOOL).cast(), 0);
    enabled != 0
}

unsafe fn apply_dwm_chrome(hwnd: HWND, dark: bool) {
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

unsafe fn show_signal(st: &mut UiState, control: Control) {
    if st.motion {
        st.signal = Some((control, Instant::now()));
        SetTimer(st.hwnd, TIMER_SIGNAL, 42, ptr::null());
        invalidate(st);
    }
}

unsafe fn activate(st: &mut UiState, control: Control) {
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

fn seg_hit(r: RectF, s: f32, x: f32, y: f32) -> i32 {
    if !rect_contains(r, x, y) {
        return -1;
    }
    let pad = 4.0 * s;
    let sw = (r.w - pad * 2.0) * 0.5;
    if x < r.x + pad + sw { 0 } else { 1 }
}

fn any_hover(st: &UiState) -> bool {
    st.hover_stage
        || st.hover_action >= 0
        || st.hover_preset >= 0
        || st.hover_follow
        || st.hover_startup
        || st.hover_reset
        || st.hover_done
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
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

unsafe fn make_tray_data(hwnd: HWND, hicon: HICON) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as DWORD;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY;
    nid.hIcon = hicon;
    for (i, ch) in APP_NAME.encode_utf16().enumerate().take(127) {
        nid.szTip[i] = ch;
    }
    nid
}

unsafe fn add_tray_icon(hwnd: HWND, hicon: HICON) -> bool {
    let nid = make_tray_data(hwnd, hicon);
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

fn main() {
    unsafe {
        let background = std::env::args().skip(1).any(|a| a == "--background");

        let mname = wide("PinchZoomSingleInstance");
        let mutex = CreateMutexW(ptr::null(), 1, mname.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if !background {
                let cls = wide(WND_CLASS);
                let existing = FindWindowW(cls.as_ptr(), ptr::null());
                if existing != 0 {
                    PostMessageW(existing, WM_APP_SHOW, 0, 0);
                }
            }
            if mutex != 0 {
                CloseHandle(mutex);
            }
            return;
        }

        if SetProcessDpiAwarenessContext(-4isize) == 0 {
            SetProcessDPIAware();
        }

        let hdc_screen = GetDC(0);
        let dpi = GetDeviceCaps(hdc_screen, LOGPIXELSX);
        ReleaseDC(0, hdc_screen);
        let scale = if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 };

        let mut gdi_token: usize = 0;
        let gdi_in = GdiplusStartupInput {
            version: 1,
            debug_event_callback: 0,
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        GdiplusStartup(&mut gdi_token, &gdi_in, ptr::null_mut());

        let mut mag_ok = MagInitialize() != 0;
        if !mag_ok && background {
            for _ in 0..20 {
                Sleep(500);
                if MagInitialize() != 0 {
                    mag_ok = true;
                    break;
                }
            }
        }
        if !mag_ok {
            if !background {
                let msg = wide("Could not start the Windows magnification API.");
                let cap = wide(APP_NAME);
                MessageBoxW(0, msg.as_ptr(), cap.as_ptr(), 0);
            }
            GdiplusShutdown(gdi_token);
            CloseHandle(mutex);
            return;
        }

        let tc_name = wide("TaskbarCreated");
        let taskbar_created_msg = RegisterWindowMessageW(tc_name.as_ptr());

        let mut init_state = UiState::new(scale);
        init_state.taskbar_created_msg = taskbar_created_msg;
        init_state.dark = system_dark();
        init_state.motion = motion_enabled();
        load_settings(&mut init_state);
        ensure_startup_default();
        init_state.startup_on = startup_enabled();
        init_state.action_cur = init_state.action as f32;
        init_state.preset_cur = init_state.preset() as f32;
        init_state.follow_k = if init_state.follow_mouse { 1.0 } else { 0.0 };
        init_state.startup_k = if init_state.startup_on { 1.0 } else { 0.0 };
        init_state.hicon = draw::make_icon((32.0 * scale).round() as i32);
        init_state.cursor_arrow = LoadCursorW(0, IDC_ARROW as *const u16);
        init_state.cursor_hand = LoadCursorW(0, IDC_HAND as *const u16);

        let hinst = GetModuleHandleW(ptr::null());
        let cls = wide(WND_CLASS);
        let title = wide(APP_NAME);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: init_state.hicon,
            hCursor: init_state.cursor_arrow,
            hbrBackground: 0,
            lpszMenuName: ptr::null(),
            lpszClassName: cls.as_ptr(),
            hIconSm: init_state.hicon,
        };
        RegisterClassExW(&wc);

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let ex_style = WS_EX_LAYERED;
        let mut wr = RECT {
            left: 0,
            top: 0,
            right: (CLIENT_W * scale).round() as i32,
            bottom: (CLIENT_H * scale).round() as i32,
        };
        AdjustWindowRectEx(&mut wr, style, 0, ex_style);
        let win_w = wr.right - wr.left;
        let win_h = wr.bottom - wr.top;
        let sx = (GetSystemMetrics(SM_CXSCREEN) - win_w) / 2;
        let sy = (GetSystemMetrics(SM_CYSCREEN) - win_h) / 2;

        STATE.with(|cell| *cell.borrow_mut() = Some(init_state));

        let hwnd = CreateWindowExW(
            ex_style,
            cls.as_ptr(),
            title.as_ptr(),
            style,
            sx,
            sy,
            win_w,
            win_h,
            0,
            0,
            hinst,
            ptr::null_mut(),
        );
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        apply_dwm_chrome(hwnd, with_state(|st| st.dark));

        let hicon = with_state(|st| {
            st.hwnd = hwnd;
            st.hicon
        });
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL, hicon);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG, hicon);

        // Install hooks outside STATE borrow so early input events can never re-enter RefCell.
        let m_hook = SetWindowsHookExW(WH_MOUSE_LL, mouse_hook_proc, hinst, 0);
        let k_hook = SetWindowsHookExW(WH_KEYBOARD_LL, kb_hook_proc, hinst, 0);
        with_state(|st| {
            st.mouse_hook = m_hook;
            st.kb_hook = k_hook;
        });

        let mut tray_ok = add_tray_icon(hwnd, hicon);
        if !tray_ok && background {
            for _ in 0..10 {
                Sleep(500);
                if add_tray_icon(hwnd, hicon) {
                    tray_ok = true;
                    break;
                }
            }
        }
        let _ = tray_ok;

        if !background {
            show_settings();
        }

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let nid = make_tray_data(hwnd, hicon);
        Shell_NotifyIconW(NIM_DELETE, &nid);
        with_state(|st| {
            if st.mouse_hook != 0 { UnhookWindowsHookEx(st.mouse_hook); }
            if st.kb_hook != 0 { UnhookWindowsHookEx(st.kb_hook); }
            if st.hicon != 0 { DestroyIcon(st.hicon); }
        });
        MagSetFullscreenTransform(1.0, 0, 0);
        MagUninitialize();
        GdiplusShutdown(gdi_token);
        if mutex != 0 {
            CloseHandle(mutex);
        }
    }
}
