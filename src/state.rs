//! The app's one piece of mutable state (UiState) and the thread-local that holds it.
//! Hooks, timers and the window proc all reach it through with_state / try_with_state.

use super::*;

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
    pub touchpad: Option<touchpad::Touchpad>,
    pub pinch_apps: BTreeSet<String>,

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
    pub(crate) fn new(scale: f32) -> Self {
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
            touchpad: touchpad::Touchpad::new(),
            pinch_apps: BTreeSet::new(),
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

    pub(crate) fn update_active(&mut self) {
        self.hot_mods = self.active_mods();
    }
}

thread_local! {
    pub(crate) static STATE: RefCell<Option<UiState>> = const { RefCell::new(None) };
}

pub(crate) fn try_with_state<R>(f: impl FnOnce(&mut UiState) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut opt = cell.try_borrow_mut().ok()?;
        opt.as_mut().map(f)
    })
}

pub(crate) fn with_state<R>(f: impl FnOnce(&mut UiState) -> R) -> R {
    STATE.with(|cell| {
        let mut opt = cell.borrow_mut();
        f(opt.as_mut().expect("state initialized"))
    })
}
