use std::ffi::c_void;
use std::mem::size_of;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use crate::platform::abgr_to_hex;

fn read_user_dword(subkey: PCWSTR, value: PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: the output buffer is a local u32 whose size is passed along.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            value,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Apps mode chosen in Settings > Personalization > Colors (light unless set to dark).
pub fn system_prefers_light() -> bool {
    read_user_dword(w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"), w!("AppsUseLightTheme"))
        .map(|value| value != 0)
        .unwrap_or(true)
}

/// The accent color chosen in Settings > Personalization > Colors.
pub fn system_accent_color() -> Option<String> {
    read_user_dword(w!("Software\\Microsoft\\Windows\\DWM"), w!("AccentColor")).map(abgr_to_hex)
}
