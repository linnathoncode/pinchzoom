//! Settings persistence in the registry: hotkey, action, per-app pinch list,
//! and the "start with Windows" entry.

use super::*;

pub(crate) const REG_NAME: &str = "PinchZoom";
pub(crate) const SETTINGS_KEY: &str = "Software\\PinchZoom";
pub(crate) const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
pub(crate) const STARTUP_APPROVED_KEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";

pub(crate) unsafe fn reg_read_dword(subkey: &str, name: &str) -> Option<i32> {
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

pub(crate) unsafe fn reg_write_dword(subkey: &str, name: &str, value: i32) {
    reg_write_value(subkey, name, REG_DWORD, &value.to_ne_bytes());
}

pub(crate) unsafe fn reg_write_value(subkey: &str, name: &str, kind: DWORD, value: &[u8]) {
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
    RegSetValueExW(hkey, vn.as_ptr(), 0, kind, value.as_ptr(), value.len() as DWORD);
    RegCloseKey(hkey);
}

pub(crate) unsafe fn load_pinch_apps() -> BTreeSet<String> {
    let mut key = 0;
    let subkey = wide(SETTINGS_KEY);
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key) != 0 {
        return BTreeSet::new();
    }
    let name = wide("TouchpadApps");
    let mut kind = 0;
    let mut size = 0;
    let status = RegQueryValueExW(key, name.as_ptr(), ptr::null(), &mut kind, ptr::null_mut(), &mut size);
    let mut apps = BTreeSet::new();
    if status == 0 && kind == REG_MULTI_SZ && size % 2 == 0 && size <= 4 * 1024 * 1024 {
        let mut data = vec![0u16; size as usize / 2];
        let status = RegQueryValueExW(
            key, name.as_ptr(), ptr::null(), &mut kind, data.as_mut_ptr().cast(), &mut size,
        );
        if status == 0 && kind == REG_MULTI_SZ {
            for path in data[..size as usize / 2].split(|&ch| ch == 0).take_while(|path| !path.is_empty()) {
                apps.insert(String::from_utf16_lossy(path).to_lowercase());
            }
        }
    }
    RegCloseKey(key);
    apps
}

pub(crate) unsafe fn save_pinch_apps(apps: &BTreeSet<String>) {
    let data: Vec<u8> = apps.iter().flat_map(|path| path.encode_utf16().chain([0]))
        .chain([0, 0])
        .flat_map(u16::to_le_bytes)
        .collect();
    reg_write_value(SETTINGS_KEY, "TouchpadApps", REG_MULTI_SZ, &data);
}

pub(crate) unsafe fn load_settings(st: &mut UiState) {
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
    st.pinch_apps = load_pinch_apps();
    st.update_active();
}

pub(crate) unsafe fn save_settings(st: &mut UiState) {
    st.update_active();
    reg_write_dword(SETTINGS_KEY, "CustomModifiers", st.custom_mods);
    reg_write_dword(SETTINGS_KEY, "UseCustom", if st.use_custom { 1 } else { 0 });
    reg_write_dword(SETTINGS_KEY, "Action", st.action);
    reg_write_dword(SETTINGS_KEY, "FollowMouse", if st.follow_mouse { 1 } else { 0 });
}

pub(crate) unsafe fn startup_enabled() -> bool {
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

pub(crate) unsafe fn exe_path() -> String {
    let mut buf = [0u16; 1024];
    let len = GetModuleFileNameW(0, buf.as_mut_ptr(), buf.len() as DWORD) as usize;
    String::from_utf16_lossy(&buf[..len])
}

pub(crate) unsafe fn set_startup(on: bool) {
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

pub(crate) unsafe fn ensure_startup_default() {
    if reg_read_dword(SETTINGS_KEY, "StartupInit").is_none() {
        set_startup(true);
        reg_write_dword(SETTINGS_KEY, "StartupInit", 1);
    }
}
