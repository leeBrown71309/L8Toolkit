use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::platform::ShellVerb;

/// ShellExecute returns a value greater than 32 on success.
const SHELL_EXECUTE_SUCCESS_THRESHOLD: isize = 32;
const ERROR_CANCELLED_BY_USER: isize = 5;

/// Opens a path, URL, protocol (ms-settings:) or AppsFolder id with its
/// default handler, exactly like double-clicking it in Explorer.
pub fn shell_open(target: &str, verb: ShellVerb) -> Result<(), String> {
    let operation = match verb {
        ShellVerb::Open => w!("open"),
        ShellVerb::RunAsAdmin => w!("runas"),
    };
    // SAFETY: all strings outlive the call; no window handle is required.
    let result = unsafe {
        ShellExecuteW(
            None,
            operation,
            &HSTRING::from(target),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code > SHELL_EXECUTE_SUCCESS_THRESHOLD {
        return Ok(());
    }
    if code == ERROR_CANCELLED_BY_USER && verb == ShellVerb::RunAsAdmin {
        return Err("Elevation was cancelled".to_string());
    }
    Err(format!("Windows could not open '{target}' (code {code})"))
}

/// Opens Explorer with the item selected.
pub fn reveal_in_file_manager(path: &str) -> Result<(), String> {
    // `raw_arg` keeps Explorer's unusual `/select,"path"` syntax intact.
    Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot open Explorer: {error}"))
}

pub fn lock_workstation() -> Result<(), String> {
    // SAFETY: no arguments, documented to be callable from any desktop app.
    unsafe { LockWorkStation() }.map_err(|error| format!("Cannot lock the session: {error}"))
}
