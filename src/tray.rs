//! The notification-area icon and its popup menu.

use super::*;

pub(crate) const MENU_SETTINGS: usize = 1001;
pub(crate) const MENU_RESET: usize = 1002;
pub(crate) const MENU_QUIT: usize = 1003;
pub(crate) const MENU_PINCH_APP: usize = 1100;

pub(crate) unsafe fn show_tray_menu() {
    let (hwnd, hot_mods, action, touchpad_supported, selected) = with_state(|st| {
        (st.hwnd, st.hot_mods, st.action, st.touchpad.is_some(), st.pinch_apps.clone())
    });
    let menu = CreatePopupMenu();
    let summary = wide(&format_zoom_summary(hot_mods, action));
    AppendMenuW(menu, MF_STRING | MF_GRAYED | MF_DISABLED, 0, summary.as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
    let s_set = wide("Settings");
    let s_rst = wide("Reset zoom");
    let s_quit = wide(&format!("Quit {}", APP_NAME));
    AppendMenuW(menu, MF_STRING, MENU_SETTINGS, s_set.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_RESET, s_rst.as_ptr());
    let mut app_paths = Vec::new();
    if touchpad_supported {
        let apps_menu = CreatePopupMenu();
        if apps_menu != 0 {
            let mut apps = touchpad::open_apps(hwnd);
            apps.extend(selected.iter().cloned());
            app_paths = apps.into_iter().collect();
            let name = |path: &str| path.rsplit('\\').next().unwrap_or(path).to_string();
            app_paths.sort_by_cached_key(|path| (name(path), path.clone()));
            for (index, path) in app_paths.iter().enumerate() {
                let app_name = name(path);
                let duplicate = app_paths.get(index.wrapping_sub(1)).is_some_and(|p| name(p) == app_name)
                    || app_paths.get(index + 1).is_some_and(|p| name(p) == app_name);
                let label = if duplicate { path } else { &app_name };
                let label = wide(&label.replace('&', "&&"));
                let flags = if selected.contains(path) { MF_CHECKED } else { 0 };
                AppendMenuW(apps_menu, MF_STRING | flags, MENU_PINCH_APP + index, label.as_ptr());
            }
            if app_paths.is_empty() {
                let label = wide("Open an app to add it");
                AppendMenuW(apps_menu, MF_STRING | MF_GRAYED, 0, label.as_ptr());
            }
            let label = wide("Touchpad pinch apps");
            if AppendMenuW(menu, MF_POPUP, apps_menu as usize, label.as_ptr()) == 0 {
                DestroyMenu(apps_menu);
            }
        }
    } else {
        let label = wide("Touchpad pinch unavailable on this Windows build");
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, label.as_ptr());
    }
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
    } else if let Some(path) = cmd.checked_sub(MENU_PINCH_APP).and_then(|index| app_paths.get(index)) {
        with_state(|st| {
            if !st.pinch_apps.remove(path) {
                st.pinch_apps.insert(path.clone());
            }
            save_pinch_apps(&st.pinch_apps);
        });
    }
}

pub(crate) unsafe fn make_tray_data(hwnd: HWND, hicon: HICON) -> NOTIFYICONDATAW {
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

pub(crate) unsafe fn add_tray_icon(hwnd: HWND, hicon: HICON) -> bool {
    let nid = make_tray_data(hwnd, hicon);
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}
