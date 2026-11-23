#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HINSTANCE = isize;
pub type HMODULE = isize;
pub type HDC = isize;
pub type HBITMAP = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HBRUSH = isize;
pub type HMENU = isize;
pub type HHOOK = isize;
pub type HKEY = isize;
pub type HANDLE = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;

pub type WndProc = unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;
pub type HookProc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

pub const WH_KEYBOARD_LL: i32 = 13;
pub const WH_MOUSE_LL: i32 = 14;

pub const WM_DESTROY: UINT = 0x0002;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_SETTINGCHANGE: UINT = 0x001A;
pub const WM_THEMECHANGED: UINT = 0x031A;
pub const WM_SETCURSOR: UINT = 0x0020;
pub const WM_SETICON: UINT = 0x0080;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_SYSKEYUP: UINT = 0x0105;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MBUTTONDOWN: UINT = 0x0207;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_MOUSELEAVE: UINT = 0x02A3;
pub const WM_APP: UINT = 0x8000;

pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_EX_LAYERED: DWORD = 0x00080000;

pub const CS_HREDRAW: UINT = 0x0002;
pub const CS_VREDRAW: UINT = 0x0001;

pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_RESTORE: i32 = 9;

pub const LWA_ALPHA: DWORD = 0x00000002;
pub const ICON_SMALL: WPARAM = 0;
pub const ICON_BIG: WPARAM = 1;

pub const IDC_ARROW: usize = 32512;
pub const IDC_HAND: usize = 32649;
pub const HTCLIENT: isize = 1;

pub const VK_TAB: i32 = 0x09;
pub const VK_SPACE: i32 = 0x20;
pub const VK_RETURN: i32 = 0x0D;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_LWIN: i32 = 0x5B;
pub const VK_RWIN: i32 = 0x5C;
pub const VK_MULTIPLY: u32 = 0x6A;
pub const VK_ADD: u32 = 0x6B;
pub const VK_SUBTRACT: u32 = 0x6D;

pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const LOGPIXELSX: i32 = 88;
pub const SRCCOPY: DWORD = 0x00CC0020;
pub const SPI_GETCLIENTAREAANIMATION: UINT = 0x1042;
pub const TME_LEAVE: DWORD = 0x00000002;

pub const NIM_ADD: DWORD = 0x00000000;
pub const NIM_DELETE: DWORD = 0x00000002;
pub const NIF_MESSAGE: UINT = 0x00000001;
pub const NIF_ICON: UINT = 0x00000002;
pub const NIF_TIP: UINT = 0x00000004;

pub const MF_STRING: UINT = 0x00000000;
pub const MF_GRAYED: UINT = 0x00000001;
pub const MF_DISABLED: UINT = 0x00000002;
pub const MF_SEPARATOR: UINT = 0x00000800;
pub const TPM_RIGHTBUTTON: UINT = 0x0002;
pub const TPM_BOTTOMALIGN: UINT = 0x0020;
pub const TPM_RETURNCMD: UINT = 0x0100;

pub const HKEY_CURRENT_USER: HKEY = 0x80000001u32 as isize;
pub const KEY_READ: DWORD = 0x20019;
pub const KEY_WRITE: DWORD = 0x20006;
pub const REG_SZ: DWORD = 1;
pub const REG_BINARY: DWORD = 3;
pub const REG_DWORD: DWORD = 4;
pub const ERROR_ALREADY_EXISTS: DWORD = 183;

pub const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
pub const DWMWA_BORDER_COLOR: DWORD = 34;
pub const DWMWA_CAPTION_COLOR: DWORD = 35;
pub const DWMWA_TEXT_COLOR: DWORD = 36;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
pub struct MSLLHOOKSTRUCT {
    pub pt: POINT,
    pub mouseData: DWORD,
    pub flags: DWORD,
    pub time: DWORD,
    pub extra: usize,
}

#[repr(C)]
pub struct KBDLLHOOKSTRUCT {
    pub vkCode: DWORD,
    pub scanCode: DWORD,
    pub flags: DWORD,
    pub time: DWORD,
    pub extra: usize,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: Option<WndProc>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: DWORD,
    pub dwFlags: DWORD,
    pub hwndTrack: HWND,
    pub dwHoverTime: DWORD,
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: DWORD,
    pub hWnd: HWND,
    pub uID: UINT,
    pub uFlags: UINT,
    pub uCallbackMessage: UINT,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: DWORD,
    pub dwStateMask: DWORD,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: UINT,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: DWORD,
    pub guidItem: [u8; 16],
    pub hBalloonIcon: HICON,
}

#[repr(C)]
pub struct GdiplusStartupInput {
    pub version: u32,
    pub debug_event_callback: usize,
    pub suppress_background_thread: i32,
    pub suppress_external_codecs: i32,
}

#[link(name = "user32")]
extern "system" {
    pub fn SetWindowsHookExW(id: i32, lpfn: HookProc, hmod: HMODULE, tid: DWORD) -> HHOOK;
    pub fn CallNextHookEx(hhk: HHOOK, nCode: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;
    pub fn SystemParametersInfoW(action: UINT, param: UINT, value: *mut c_void, flags: UINT) -> BOOL;
    pub fn GetAsyncKeyState(vKey: i32) -> i16;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn keybd_event(bVk: u8, bScan: u8, dwFlags: DWORD, dwExtraInfo: usize);
    pub fn SetProcessDPIAware() -> BOOL;
    pub fn SetProcessDpiAwarenessContext(value: isize) -> BOOL;
    pub fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hwnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn PostMessageW(hwnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn SendMessageW(hwnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    pub fn ShowWindow(hwnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    pub fn SetForegroundWindow(hwnd: HWND) -> BOOL;
    pub fn SetLayeredWindowAttributes(hwnd: HWND, crKey: DWORD, bAlpha: u8, dwFlags: DWORD) -> BOOL;
    pub fn InvalidateRect(hwnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn BeginPaint(hwnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hwnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetClientRect(hwnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn AdjustWindowRectEx(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL, dwExStyle: DWORD) -> BOOL;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn SetTimer(hwnd: HWND, id: usize, elapse: UINT, proc: *const c_void) -> usize;
    pub fn KillTimer(hwnd: HWND, id: usize) -> BOOL;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    pub fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    pub fn TrackMouseEvent(lpEventTrack: *mut TRACKMOUSEEVENT) -> BOOL;
    pub fn DestroyIcon(hIcon: HICON) -> BOOL;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(hMenu: HMENU, uFlags: UINT, uIDNewItem: usize, lpNewItem: *const u16) -> BOOL;
    pub fn TrackPopupMenuEx(hMenu: HMENU, uFlags: UINT, x: i32, y: i32, hwnd: HWND, lptpm: *const c_void) -> BOOL;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn MessageBoxW(hwnd: HWND, text: *const u16, caption: *const u16, uType: UINT) -> i32;
    pub fn RegisterWindowMessageW(lpString: *const u16) -> UINT;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HMODULE;
    pub fn GetModuleFileNameW(hModule: HMODULE, lpFilename: *mut u16, nSize: DWORD) -> DWORD;
    pub fn CreateMutexW(lpAttrs: *const c_void, bInitialOwner: BOOL, lpName: *const u16) -> HANDLE;
    pub fn GetLastError() -> DWORD;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn Sleep(dwMilliseconds: DWORD);
}

#[link(name = "gdi32")]
extern "system" {
    pub fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn CreateCompatibleBitmap(hdc: HDC, cx: i32, cy: i32) -> HBITMAP;
    pub fn SelectObject(hdc: HDC, h: isize) -> isize;
    pub fn DeleteObject(ho: isize) -> BOOL;
    pub fn BitBlt(
        hdc: HDC,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdcSrc: HDC,
        x1: i32,
        y1: i32,
        rop: DWORD,
    ) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: DWORD, lpData: *const NOTIFYICONDATAW) -> BOOL;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        ulOptions: DWORD,
        samDesired: DWORD,
        phkResult: *mut HKEY,
    ) -> i32;
    pub fn RegCreateKeyExW(
        hKey: HKEY,
        lpSubKey: *const u16,
        reserved: DWORD,
        lpClass: *const u16,
        dwOptions: DWORD,
        samDesired: DWORD,
        lpSec: *const c_void,
        phkResult: *mut HKEY,
        lpdwDisp: *mut DWORD,
    ) -> i32;
    pub fn RegQueryValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        lpReserved: *const DWORD,
        lpType: *mut DWORD,
        lpData: *mut u8,
        lpcbData: *mut DWORD,
    ) -> i32;
    pub fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: *const u16,
        reserved: DWORD,
        dwType: DWORD,
        lpData: *const u8,
        cbData: DWORD,
    ) -> i32;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: *const u16) -> i32;
    pub fn RegCloseKey(hKey: HKEY) -> i32;
}

#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: HWND, attr: DWORD, pv: *const c_void, cb: DWORD) -> i32;
}

#[link(name = "Magnification")]
extern "system" {
    pub fn MagInitialize() -> BOOL;
    pub fn MagUninitialize() -> BOOL;
    pub fn MagSetFullscreenTransform(magLevel: f32, xOffset: i32, yOffset: i32) -> BOOL;
}

#[link(name = "gdiplus")]
extern "system" {
    pub fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut c_void) -> i32;
    pub fn GdiplusShutdown(token: usize);
    pub fn GdipCreateFromHDC(hdc: HDC, graphics: *mut *mut c_void) -> i32;
    pub fn GdipDeleteGraphics(graphics: *mut c_void) -> i32;
    pub fn GdipSetSmoothingMode(graphics: *mut c_void, mode: i32) -> i32;
    pub fn GdipSetPixelOffsetMode(graphics: *mut c_void, mode: i32) -> i32;
    pub fn GdipSetTextRenderingHint(graphics: *mut c_void, mode: i32) -> i32;
    pub fn GdipGraphicsClear(graphics: *mut c_void, color: u32) -> i32;
    pub fn GdipSaveGraphics(graphics: *mut c_void, state: *mut u32) -> i32;
    pub fn GdipRestoreGraphics(graphics: *mut c_void, state: u32) -> i32;
    pub fn GdipSetClipPath(graphics: *mut c_void, path: *mut c_void, combine_mode: i32) -> i32;
    pub fn GdipTranslateWorldTransform(graphics: *mut c_void, dx: f32, dy: f32, order: i32) -> i32;
    pub fn GdipScaleWorldTransform(graphics: *mut c_void, sx: f32, sy: f32, order: i32) -> i32;

    pub fn GdipCreatePath(brush_mode: i32, path: *mut *mut c_void) -> i32;
    pub fn GdipDeletePath(path: *mut c_void) -> i32;
    pub fn GdipAddPathArc(path: *mut c_void, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> i32;
    pub fn GdipAddPathBezier(
        path: *mut c_void,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
    ) -> i32;
    pub fn GdipAddPathEllipse(path: *mut c_void, x: f32, y: f32, w: f32, h: f32) -> i32;
    pub fn GdipAddPathRectangle(path: *mut c_void, x: f32, y: f32, w: f32, h: f32) -> i32;
    pub fn GdipClosePathFigure(path: *mut c_void) -> i32;

    pub fn GdipCreateSolidFill(color: u32, brush: *mut *mut c_void) -> i32;
    pub fn GdipCreateLineBrush(
        pt1: *const PointF,
        pt2: *const PointF,
        c1: u32,
        c2: u32,
        wrap: i32,
        brush: *mut *mut c_void,
    ) -> i32;
    pub fn GdipSetLinePresetBlend(brush: *mut c_void, blend: *const u32, positions: *const f32, count: i32) -> i32;
    pub fn GdipCreatePathGradientFromPath(path: *mut c_void, poly_grad: *mut *mut c_void) -> i32;
    pub fn GdipSetPathGradientCenterColor(brush: *mut c_void, color: u32) -> i32;
    pub fn GdipSetPathGradientSurroundColorsWithCount(
        brush: *mut c_void,
        colors: *const u32,
        count: *mut i32,
    ) -> i32;
    pub fn GdipDeleteBrush(brush: *mut c_void) -> i32;

    pub fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut *mut c_void) -> i32;
    pub fn GdipCreatePen2(brush: *mut c_void, width: f32, unit: i32, pen: *mut *mut c_void) -> i32;
    pub fn GdipSetPenStartCap(pen: *mut c_void, cap: i32) -> i32;
    pub fn GdipSetPenEndCap(pen: *mut c_void, cap: i32) -> i32;
    pub fn GdipSetPenLineJoin(pen: *mut c_void, join: i32) -> i32;
    pub fn GdipSetPenDashStyle(pen: *mut c_void, style: i32) -> i32;
    pub fn GdipDeletePen(pen: *mut c_void) -> i32;

    pub fn GdipFillPath(graphics: *mut c_void, brush: *mut c_void, path: *mut c_void) -> i32;
    pub fn GdipDrawPath(graphics: *mut c_void, pen: *mut c_void, path: *mut c_void) -> i32;
    pub fn GdipFillRectangle(graphics: *mut c_void, brush: *mut c_void, x: f32, y: f32, w: f32, h: f32) -> i32;
    pub fn GdipFillEllipse(graphics: *mut c_void, brush: *mut c_void, x: f32, y: f32, w: f32, h: f32) -> i32;
    pub fn GdipFillPolygon(
        graphics: *mut c_void,
        brush: *mut c_void,
        points: *const PointF,
        count: i32,
        fill_mode: i32,
    ) -> i32;
    pub fn GdipDrawLine(graphics: *mut c_void, pen: *mut c_void, x1: f32, y1: f32, x2: f32, y2: f32) -> i32;

    pub fn GdipCreateFontFamilyFromName(
        name: *const u16,
        collection: *mut c_void,
        family: *mut *mut c_void,
    ) -> i32;
    pub fn GdipDeleteFontFamily(family: *mut c_void) -> i32;
    pub fn GdipCreateFont(
        family: *mut c_void,
        em_size: f32,
        style: i32,
        unit: i32,
        font: *mut *mut c_void,
    ) -> i32;
    pub fn GdipDeleteFont(font: *mut c_void) -> i32;

    pub fn GdipStringFormatGetGenericTypographic(format: *mut *mut c_void) -> i32;
    pub fn GdipCloneStringFormat(src: *mut c_void, dst: *mut *mut c_void) -> i32;
    pub fn GdipSetStringFormatAlign(format: *mut c_void, align: i32) -> i32;
    pub fn GdipSetStringFormatLineAlign(format: *mut c_void, align: i32) -> i32;
    pub fn GdipSetStringFormatFlags(format: *mut c_void, flags: i32) -> i32;
    pub fn GdipDeleteStringFormat(format: *mut c_void) -> i32;

    pub fn GdipDrawString(
        graphics: *mut c_void,
        text: *const u16,
        len: i32,
        font: *mut c_void,
        layout: *const RectF,
        format: *mut c_void,
        brush: *mut c_void,
    ) -> i32;
    pub fn GdipMeasureString(
        graphics: *mut c_void,
        text: *const u16,
        len: i32,
        font: *mut c_void,
        layout: *const RectF,
        format: *mut c_void,
        bounds: *mut RectF,
        chars: *mut i32,
        lines: *mut i32,
    ) -> i32;

    pub fn GdipCreateBitmapFromScan0(
        w: i32,
        h: i32,
        stride: i32,
        format: i32,
        scan0: *mut u8,
        bmp: *mut *mut c_void,
    ) -> i32;
    pub fn GdipGetImageGraphicsContext(image: *mut c_void, graphics: *mut *mut c_void) -> i32;
    pub fn GdipCreateHICONFromBitmap(bmp: *mut c_void, hicon: *mut HICON) -> i32;
    pub fn GdipDisposeImage(image: *mut c_void) -> i32;
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn colorref(argb: u32) -> DWORD {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    r | (g << 8) | (b << 16)
}
