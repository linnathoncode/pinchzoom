use std::collections::BTreeSet;
use std::ffi::c_void;
use std::mem;

use crate::win32::*;

pub const FRAME_MS: u32 = 15;
pub const WHEEL_STEP: i32 = 120;
pub const NUMPAD_STEP: i32 = 60;

const FRAMES: i32 = 10;
const WIDTH: i32 = 40_000;
const HEIGHT: i32 = 24_000;
const PT_TOUCHPAD: u32 = 5;
const CONTACT: u32 = 0x0006;
const CONFIDENCE: u32 = 0x4000;

#[repr(C)]
struct DeviceParams {
    pointer_type: u32,
    max_count: u32,
    feedback_mode: u32,
    monitor: HANDLE,
    width: u32,
    height: u32,
    options: u32,
}

#[repr(C)]
struct PointerInfo {
    pointer_type: u32,
    id: u32,
    frame_id: u32,
    flags: u32,
    source_device: HANDLE,
    target: HWND,
    pixel: POINT,
    himetric: POINT,
    pixel_raw: POINT,
    himetric_raw: POINT,
    time: u32,
    history_count: u32,
    input_data: i32,
    key_states: u32,
    performance_count: u64,
    button_change_type: u32,
}

#[repr(C)]
struct TouchInfo {
    pointer: PointerInfo,
    flags: u32,
    mask: u32,
    contact: RECT,
    contact_raw: RECT,
    orientation: u32,
    pressure: u32,
}

#[repr(C)]
struct PointerTypeInfo {
    pointer_type: u32,
    // POINTER_TOUCH_INFO is the largest member of the Win32 union.
    touch: TouchInfo,
}

type CreateDevice = unsafe extern "system" fn(*const DeviceParams) -> HANDLE;
type Inject = unsafe extern "system" fn(HANDLE, *const PointerTypeInfo, u32) -> BOOL;
type DestroyDevice = unsafe extern "system" fn(HANDLE);

struct Api {
    create: CreateDevice,
    inject: Inject,
    destroy: DestroyDevice,
}

impl Api {
    fn load() -> Option<Self> {
        let name = wide("user32.dll");
        unsafe {
            let module = GetModuleHandleW(name.as_ptr());
            let create = GetProcAddress(module, c"CreateSyntheticPointerDevice2".as_ptr().cast());
            let inject = GetProcAddress(module, c"InjectSyntheticPointerInput".as_ptr().cast());
            let destroy = GetProcAddress(module, c"DestroySyntheticPointerDevice".as_ptr().cast());
            if create.is_null() || inject.is_null() || destroy.is_null() {
                return None;
            }
            Some(Self {
                create: mem::transmute::<*const c_void, CreateDevice>(create),
                inject: mem::transmute::<*const c_void, Inject>(inject),
                destroy: mem::transmute::<*const c_void, DestroyDevice>(destroy),
            })
        }
    }
}

struct Stroke {
    window: HWND,
    inner: i32,
    outer: i32,
    frame: i32,
}

impl Stroke {
    fn new(window: HWND, delta: i32) -> Self {
        let ratio = 1.1_f64.powf(f64::from(delta.abs()) / f64::from(WHEEL_STEP));
        // A wide starting span gives small changes enough travel to trigger pinch recognition.
        let inner = 16_000.min((18_500.0 / ratio) as i32);
        let outer = (f64::from(inner) * ratio).round() as i32;
        let (inner, outer) = if delta > 0 { (inner, outer) } else { (outer, inner) };
        Self { window, inner, outer, frame: 0 }
    }

    fn radius(&self, frame: i32) -> i32 {
        self.inner + (self.outer - self.inner) * frame / FRAMES
    }
}

pub struct Touchpad {
    api: Api,
    device: HANDLE,
    time: u32,
    stroke: Option<Stroke>,
    target: HWND,
    pending: i32,
}

impl Touchpad {
    pub fn new() -> Option<Self> {
        Some(Self { api: Api::load()?, device: 0, time: 1, stroke: None, target: 0, pending: 0 })
    }

    pub fn ready(&mut self) -> bool {
        if self.device == 0 {
            let params = DeviceParams {
                pointer_type: PT_TOUCHPAD,
                max_count: 2,
                feedback_mode: 3,
                monitor: 0,
                width: WIDTH as u32,
                height: HEIGHT as u32,
                options: 3, // SDCO_PHYSICAL_SIZE | SDCO_TOUCHPAD_GESTURE_ONLY
            };
            self.device = unsafe { (self.api.create)(&params) };
            self.time = 1;
        }
        self.device != 0
    }

    pub fn queue(&mut self, window: HWND, delta: i32) {
        if self.target != window {
            self.target = window;
            self.pending = 0;
        }
        self.pending = self.pending.saturating_add(delta);
    }

    pub fn active(&self) -> bool {
        self.stroke.is_some() || self.pending.unsigned_abs() >= NUMPAD_STEP as u32
    }

    pub fn tick(&mut self, window: HWND) {
        if self.target != window {
            self.pending = 0;
        }
        if self.stroke.is_none() && self.active() {
            let delta = self.pending.clamp(-4 * WHEEL_STEP, 4 * WHEEL_STEP);
            self.pending -= delta;
            self.stroke = Some(Stroke::new(window, delta));
        }
        if let Some(mut stroke) = self.stroke.take() {
            let contact = window == stroke.window && stroke.frame <= FRAMES;
            let frame = if contact { stroke.frame } else { (stroke.frame - 1).clamp(0, FRAMES) };
            if self.send(stroke.radius(frame), contact) && contact {
                stroke.frame += 1;
                self.stroke = Some(stroke);
            }
        }
    }

    pub fn cancel(&mut self) {
        self.pending = 0;
        if let Some(stroke) = self.stroke.take() {
            self.send(stroke.radius((stroke.frame - 1).clamp(0, FRAMES)), false);
        }
    }

    fn send(&mut self, radius: i32, contact: bool) -> bool {
        let mut points: [PointerTypeInfo; 2] = unsafe { mem::zeroed() };
        for (index, x) in [WIDTH / 2 - radius, WIDTH / 2 + radius].into_iter().enumerate() {
            let point = &mut points[index];
            point.pointer_type = PT_TOUCHPAD;
            point.touch.pointer.id = index as u32;
            point.touch.pointer.flags = CONFIDENCE | if contact { CONTACT } else { 0 };
            point.touch.pointer.himetric = POINT { x, y: HEIGHT / 2 };
            point.touch.pointer.time = self.time;
        }
        self.time = self.time.wrapping_add(FRAME_MS).max(1);
        if unsafe { (self.api.inject)(self.device, points.as_ptr(), 2) } != 0 {
            return true;
        }
        unsafe { (self.api.destroy)(self.device) };
        self.device = 0;
        self.pending = 0;
        false
    }
}

impl Drop for Touchpad {
    fn drop(&mut self) {
        if self.device != 0 {
            unsafe { (self.api.destroy)(self.device) };
        }
    }
}

pub fn window_at(point: POINT) -> HWND {
    unsafe { GetAncestor(WindowFromPoint(point), 2) }
}

pub fn cursor_window() -> HWND {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return 0;
    }
    window_at(point)
}

pub fn app_path(window: HWND) -> Option<String> {
    if window == 0 {
        return None;
    }
    unsafe {
        let mut process = 0;
        GetWindowThreadProcessId(window, &mut process);
        let handle = OpenProcess(0x1000, 0, process);
        if handle == 0 {
            return None;
        }
        let mut path = [0u16; 32_768];
        let mut size = path.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        (ok != 0).then(|| String::from_utf16_lossy(&path[..size as usize]).to_lowercase())
    }
}

pub fn open_apps(exclude: HWND) -> BTreeSet<String> {
    unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> BOOL {
        let (exclude, apps) = &mut *(data as *mut (HWND, BTreeSet<String>));
        if window != *exclude && IsWindowVisible(window) != 0 && GetWindowTextLengthW(window) > 0 {
            if let Some(path) = app_path(window) {
                apps.insert(path);
            }
        }
        1
    }

    let mut result = (exclude, BTreeSet::new());
    unsafe { EnumWindows(collect, (&mut result as *mut (HWND, BTreeSet<String>)) as LPARAM) };
    result.1
}
