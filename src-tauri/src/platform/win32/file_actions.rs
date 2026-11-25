use std::mem::size_of;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::Duration;

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND, POINT};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::{
    SHFileOperationW, SHObjectProperties, SHOpenWithDialog, DROPFILES, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
    FOF_NOERRORUI, FOF_SILENT, FO_DELETE, OAIF_ALLOW_REGISTRATION, OAIF_EXEC, OPENASINFO, SHFILEOPSTRUCTW,
    SHOP_FILEPATH,
};

use super::com::enter_com;

const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CLIPBOARD_OPEN_ATTEMPTS: u32 = 5;
const CLIPBOARD_RETRY_DELAY: Duration = Duration::from_millis(30);

/// Opens the Windows "Properties" sheet of a file, folder or executable.
/// The sheet runs on its own shell thread, so this returns immediately.
pub fn show_properties(path: &str) -> Result<(), String> {
    let _apartment = enter_com();
    // SAFETY: plain shell call with a valid, owned path string.
    let shown = unsafe { SHObjectProperties(None, SHOP_FILEPATH, &HSTRING::from(path), PCWSTR::null()) };
    if shown.as_bool() {
        Ok(())
    } else {
        Err(format!("Cannot show the properties of '{path}'"))
    }
}

/// Shows the Windows "Open with" picker. The dialog is modal to the calling
/// thread, so it gets a thread of its own.
pub fn open_with_dialog(path: &str) -> Result<(), String> {
    let path = path.to_string();
    thread::Builder::new()
        .name("open-with".into())
        .spawn(move || {
            let _apartment = enter_com();
            let file = HSTRING::from(path.as_str());
            let info = OPENASINFO {
                pcszFile: PCWSTR(file.as_ptr()),
                pcszClass: PCWSTR::null(),
                oaifInFlags: OAIF_EXEC | OAIF_ALLOW_REGISTRATION,
            };
            // SAFETY: `file` outlives the blocking dialog call.
            if let Err(error) = unsafe { SHOpenWithDialog(None, &info) } {
                log::warn!("Open with dialog failed for {path}: {error}");
            }
        })
        .map(|_| ())
        .map_err(|error| format!("Cannot show the Open with dialog: {error}"))
}

/// Sends a file or folder to the Recycle Bin (recoverable, never a hard delete).
pub fn move_to_recycle_bin(path: &str) -> Result<(), String> {
    // SHFileOperation expects a double-null-terminated list.
    let source: Vec<u16> = path.encode_utf16().chain([0, 0]).collect();
    let mut operation = SHFILEOPSTRUCTW {
        hwnd: HWND::default(),
        wFunc: FO_DELETE,
        pFrom: PCWSTR(source.as_ptr()),
        pTo: PCWSTR::null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    // SAFETY: `source` stays alive for the duration of the synchronous call.
    let code = unsafe { SHFileOperationW(&mut operation) };
    if code != 0 || operation.fAnyOperationsAborted.as_bool() {
        return Err(format!("Cannot move '{path}' to the Recycle Bin (code {code})"));
    }
    Ok(())
}

/// Puts files on the clipboard the way Explorer does (CF_HDROP), so pasting
/// in a folder copies them.
pub fn copy_files_to_clipboard(paths: &[String]) -> Result<(), String> {
    let mut file_list: Vec<u16> = Vec::new();
    for path in paths {
        file_list.extend(path.encode_utf16());
        file_list.push(0);
    }
    file_list.push(0);

    let header_size = size_of::<DROPFILES>();
    let total_size = header_size + file_list.len() * size_of::<u16>();

    // SAFETY: the global block is sized for header + list, written while
    // locked, and freed unless the clipboard takes ownership of it.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, total_size).map_err(|error| format!("Out of memory: {error}"))?;
        let destination = GlobalLock(memory) as *mut u8;
        if destination.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("Cannot lock clipboard memory".to_string());
        }
        let header = DROPFILES {
            pFiles: header_size as u32,
            pt: POINT::default(),
            fNC: false.into(),
            fWide: true.into(),
        };
        std::ptr::copy_nonoverlapping(&header as *const DROPFILES as *const u8, destination, header_size);
        std::ptr::copy_nonoverlapping(
            file_list.as_ptr() as *const u8,
            destination.add(header_size),
            file_list.len() * size_of::<u16>(),
        );
        let _ = GlobalUnlock(memory);

        if let Err(error) = open_clipboard_with_retry() {
            let _ = GlobalFree(Some(memory));
            return Err(error);
        }
        let stored = EmptyClipboard().and_then(|_| SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(memory.0))));
        let _ = CloseClipboard();
        if let Err(error) = stored {
            let _ = GlobalFree(Some(memory));
            return Err(format!("Cannot write to the clipboard: {error}"));
        }
    }
    Ok(())
}

/// Another app may hold the clipboard for a few milliseconds.
unsafe fn open_clipboard_with_retry() -> Result<(), String> {
    for _ in 0..CLIPBOARD_OPEN_ATTEMPTS {
        if OpenClipboard(None).is_ok() {
            return Ok(());
        }
        thread::sleep(CLIPBOARD_RETRY_DELAY);
    }
    Err("The clipboard is busy, try again".to_string())
}

/// Opens Windows Terminal in a folder, or the classic console if Terminal is
/// not installed.
pub fn open_in_terminal(directory: &str) -> Result<(), String> {
    if Command::new("wt.exe").args(["-d", directory]).spawn().is_ok() {
        return Ok(());
    }
    Command::new("cmd.exe")
        .raw_arg(format!("/K cd /d \"{directory}\""))
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot open a terminal: {error}"))
}
