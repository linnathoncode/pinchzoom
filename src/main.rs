#![windows_subsystem = "windows"]

//! PinchZoom: hold a modifier combo and scroll (or pinch) to magnify the screen.
//! This file is the entry point: module wiring, shared ids, single-instance start-up and the message loop.

mod draw;
mod hotkeys;
mod settings;
mod state;
mod touchpad;
mod tray;
mod win32;
mod window;
mod zoom;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ptr;
use std::time::Instant;

use draw::{paint_window, rect_contains, Control, Layout, Palette, CLIENT_H, CLIENT_W};
use win32::*;
use hotkeys::*;
use settings::*;
use state::*;
use tray::*;
use window::*;
use zoom::*;

const APP_NAME: &str = "pinchzoom";
const WND_CLASS: &str = "PinchZoomSettingsWnd";

const WM_APP_TRAY: UINT = WM_APP + 1;
const WM_APP_SHOW: UINT = WM_APP + 2;

const TIMER_ZOOM: usize = 1;
const TIMER_ANIM: usize = 2;
const TIMER_REC: usize = 3;
const TIMER_WARN: usize = 4;
const TIMER_SIGNAL: usize = 5;
const TIMER_PINCH: usize = 6;

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
