//! Clipboard history and drag-out for Recents: a listener for clipboard
//! changes, writers that put an item back, a shell drag of files or text, and
//! DPAPI to keep sensitive texts unreadable on disk.

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HGLOBAL, HLOCAL, HWND, LPARAM, LRESULT, LocalFree, WPARAM};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::{CF_DIB, CF_HDROP, CF_UNICODETEXT, DROPEFFECT_COPY, DROPEFFECT_LINK};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    DragQueryFileW, ILCreateFromPathW, ILFree, IShellItemArray, SHCreateDataObject, SHCreateShellItemArrayFromIDLists,
    SHDoDragDrop, BHID_DataObject, HDROP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId, KillTimer, RegisterClassW,
    SetTimer, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WM_TIMER, WNDCLASSW,
};

use super::com::enter_com;
use crate::platform::{ClipboardChange, ClipboardContent, ClipboardHandler};

/// Apps write several formats one after the other: read once they are done.
const SETTLE_TIMER_ID: usize = 1;
const SETTLE_DELAY_MS: u32 = 120;
const OPEN_ATTEMPTS: u32 = 6;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(25);
/// Bigger copies (a huge text, a giant bitmap) are not kept.
const MAX_TEXT_CHARS: usize = 200_000;
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
/// Password managers that copy secrets; matched against the copying process.
const PASSWORD_MANAGERS: &[&str] = &[
    "bitwarden", "1password", "keepass", "keepassxc", "dashlane", "enpass", "nordpass", "roboform", "lastpass",
    "keeper", "proton pass", "protonpass", "passbolt", "kaspersky password manager",
];

static HANDLER: Mutex<Option<ClipboardHandler>> = Mutex::new(None);
/// Sequence number of the clipboard contents this app wrote itself.
static OWN_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// Calls `handler` for every clipboard change, from a thread of its own,
/// for the rest of the app's life.
pub fn watch_clipboard(handler: ClipboardHandler) -> Result<(), String> {
    *HANDLER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(handler);
    thread::Builder::new()
        .name("clipboard-watch".into())
        .spawn(|| {
            // SAFETY: the window is created, pumped and used on this thread only.
            unsafe {
                let Ok(module) = GetModuleHandleW(None) else {
                    log::warn!("Recents: cannot find the app module");
                    return;
                };
                let class = WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: module.into(),
                    lpszClassName: w!("L8ClipboardWatch"),
                    ..Default::default()
                };
                RegisterClassW(&class);
                let window = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("L8ClipboardWatch"),
                    w!(""),
                    WINDOW_STYLE::default(),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    Some(module.into()),
                    None,
                );
                let window = match window {
                    Ok(window) => window,
                    Err(error) => {
                        log::warn!("Recents: cannot create the clipboard window: {error}");
                        return;
                    }
                };
                if let Err(error) = AddClipboardFormatListener(window) {
                    log::warn!("Recents: cannot listen to the clipboard: {error}");
                    return;
                }
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    DispatchMessageW(&message);
                }
            }
        })
        .map(|_| ())
        .map_err(|error| format!("Cannot start the clipboard watcher: {error}"))
}

unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CLIPBOARDUPDATE => {
            // SAFETY: a timer on our own window; restarting it debounces bursts.
            unsafe { SetTimer(Some(window), SETTLE_TIMER_ID, SETTLE_DELAY_MS, None) };
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == SETTLE_TIMER_ID => {
            // SAFETY: stops the one-shot timer started above.
            let _ = unsafe { KillTimer(Some(window), SETTLE_TIMER_ID) };
            // SAFETY: plain clipboard query.
            let sequence = unsafe { GetClipboardSequenceNumber() };
            if sequence != OWN_SEQUENCE.load(Ordering::SeqCst) {
                if let Some(change) = read_clipboard() {
                    if let Some(handler) = HANDLER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref() {
                        handler(change);
                    }
                }
            }
            LRESULT(0)
        }
        // SAFETY: default handling for every other message.
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn read_clipboard() -> Option<ClipboardChange> {
    let source_app = clipboard_owner_process();
    // SAFETY: the clipboard is opened, read and closed within this block.
    unsafe {
        open_clipboard().ok()?;
        let sensitive = marked_as_sensitive()
            || source_app.as_deref().is_some_and(|name| PASSWORD_MANAGERS.iter().any(|manager| name.contains(manager)));
        let content = read_files().or_else(|| read_text()).or_else(|| read_image());
        let _ = CloseClipboard();
        content.map(|content| ClipboardChange { content, sensitive, source_app })
    }
}

/// Password managers (and other careful apps) tag what they copy so that
/// clipboard history tools treat it with care.
unsafe fn marked_as_sensitive() -> bool {
    // SAFETY: registered format lookups and availability checks only.
    unsafe {
        let exclude = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        let ignore = RegisterClipboardFormatW(w!("Clipboard Viewer Ignore"));
        if IsClipboardFormatAvailable(exclude).is_ok() || IsClipboardFormatAvailable(ignore).is_ok() {
            return true;
        }
        // A DWORD 0 here means "keep out of the clipboard history".
        let history = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
        global_bytes(history).is_some_and(|bytes| bytes.len() >= 4 && bytes[..4] == [0, 0, 0, 0])
    }
}

unsafe fn read_files() -> Option<ClipboardContent> {
    // SAFETY: CF_HDROP data is an HDROP owned by the clipboard while open.
    unsafe {
        let handle = GetClipboardData(CF_HDROP.0 as u32).ok()?;
        let drop = HDROP(handle.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let files: Vec<String> = (0..count)
            .filter_map(|index| {
                let length = DragQueryFileW(drop, index, None) as usize;
                let mut buffer = vec![0u16; length + 1];
                let written = DragQueryFileW(drop, index, Some(&mut buffer)) as usize;
                (written > 0).then(|| String::from_utf16_lossy(&buffer[..written]))
            })
            .collect();
        (!files.is_empty()).then_some(ClipboardContent::Files(files))
    }
}

unsafe fn read_text() -> Option<ClipboardContent> {
    // SAFETY: CF_UNICODETEXT is a null-terminated UTF-16 global block.
    let bytes = unsafe { global_bytes(CF_UNICODETEXT.0 as u32)? };
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let end = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
    let text = String::from_utf16_lossy(&units[..end]);
    let useful = !text.trim().is_empty() && text.chars().count() <= MAX_TEXT_CHARS;
    useful.then_some(ClipboardContent::Text(text))
}

unsafe fn read_image() -> Option<ClipboardContent> {
    // SAFETY: registered format lookup and global block copies.
    unsafe {
        let png = RegisterClipboardFormatW(w!("PNG"));
        if let Some(bytes) = global_bytes(png) {
            return Some(ClipboardContent::Png(bytes));
        }
        global_bytes(CF_DIB.0 as u32).map(ClipboardContent::Dib)
    }
}

/// Copies a global-memory clipboard format, if present and not too big.
unsafe fn global_bytes(format: u32) -> Option<Vec<u8>> {
    // SAFETY: the clipboard is open; the block is locked while copied.
    unsafe {
        IsClipboardFormatAvailable(format).ok()?;
        let handle = GetClipboardData(format).ok()?;
        let memory = HGLOBAL(handle.0);
        let size = GlobalSize(memory);
        if size == 0 || size > MAX_IMAGE_BYTES {
            return None;
        }
        let pointer = GlobalLock(memory) as *const u8;
        if pointer.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(pointer, size).to_vec();
        let _ = GlobalUnlock(memory);
        Some(bytes)
    }
}

/// Executable name of the app that wrote the clipboard ("bitwarden.exe").
fn clipboard_owner_process() -> Option<String> {
    // SAFETY: plain window and process queries; the handle is closed.
    unsafe {
        let owner = GetClipboardOwner().ok()?;
        let mut process_id = 0u32;
        GetWindowThreadProcessId(owner, Some(&mut process_id));
        if process_id == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let queried = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut length);
        let _ = CloseHandle(process);
        queried.ok()?;
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Path::new(&path).file_name().map(|name| name.to_string_lossy().to_lowercase())
    }
}

/// Another app may hold the clipboard for a few milliseconds.
unsafe fn open_clipboard() -> Result<(), String> {
    for _ in 0..OPEN_ATTEMPTS {
        // SAFETY: opening without an owner window.
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Ok(());
        }
        thread::sleep(OPEN_RETRY_DELAY);
    }
    Err("The clipboard is busy, try again".to_string())
}

/// Puts formats on the clipboard; the listener ignores this write.
fn write_formats(formats: &[(u32, &[u8])]) -> Result<(), String> {
    // SAFETY: each block is allocated, filled while locked, and handed to the
    // clipboard (which then owns it) or freed on failure.
    unsafe {
        open_clipboard()?;
        if let Err(error) = EmptyClipboard() {
            let _ = CloseClipboard();
            return Err(format!("Cannot write to the clipboard: {error}"));
        }
        let mut outcome = Ok(());
        for &(format, bytes) in formats {
            if let Err(error) = set_global_data(format, bytes) {
                outcome = Err(error);
                break;
            }
        }
        OWN_SEQUENCE.store(GetClipboardSequenceNumber(), Ordering::SeqCst);
        let _ = CloseClipboard();
        outcome
    }
}

unsafe fn set_global_data(format: u32, bytes: &[u8]) -> Result<(), String> {
    // SAFETY: see `write_formats`.
    unsafe {
        let memory = global_from(bytes)?;
        if let Err(error) = SetClipboardData(format, Some(HANDLE(memory.0))) {
            let _ = GlobalFree(Some(memory));
            return Err(format!("Cannot write to the clipboard: {error}"));
        }
        Ok(())
    }
}

unsafe fn global_from(bytes: &[u8]) -> Result<HGLOBAL, String> {
    // SAFETY: the block is sized for `bytes` and written while locked.
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)).map_err(|error| format!("Out of memory: {error}"))?;
        let destination = GlobalLock(memory) as *mut u8;
        if destination.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("Cannot lock clipboard memory".to_string());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
        let _ = GlobalUnlock(memory);
        Ok(memory)
    }
}

fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
}

/// Puts text back on the clipboard (Recents › copy).
pub fn write_clipboard_text(text: &str) -> Result<(), String> {
    write_formats(&[(CF_UNICODETEXT.0 as u32, &utf16_bytes(text))])
}

/// Puts an image back on the clipboard, as a bitmap for every app and as PNG
/// for those that keep transparency.
pub fn write_clipboard_image(dib: &[u8], png: &[u8]) -> Result<(), String> {
    // SAFETY: registered format lookup.
    let png_format = unsafe { RegisterClipboardFormatW(w!("PNG")) };
    write_formats(&[(CF_DIB.0 as u32, dib), (png_format, png)])
}

/// Marks a file copy done by `copy_files_to_clipboard` as this app's own.
pub fn mark_own_clipboard_write() {
    // SAFETY: plain clipboard query.
    OWN_SEQUENCE.store(unsafe { GetClipboardSequenceNumber() }, Ordering::SeqCst);
}

/// Drags files out of the app, the way Explorer does (with its drag image).
/// Must run on the UI thread, while the mouse button is held.
pub fn start_drag_files(paths: &[String]) -> Result<(), String> {
    let _apartment = enter_com();
    // SAFETY: the item lists live until after the drag and are freed below.
    unsafe {
        let items: Vec<*const ITEMIDLIST> = paths
            .iter()
            .map(|path| ILCreateFromPathW(&HSTRING::from(path.as_str())) as *const ITEMIDLIST)
            .filter(|item| !item.is_null())
            .collect();
        if items.is_empty() {
            return Err("These files no longer exist".to_string());
        }
        let outcome = (|| {
            let array: IShellItemArray = SHCreateShellItemArrayFromIDLists(&items).map_err(|error| error.to_string())?;
            let data: IDataObject = array.BindToHandler(None, &BHID_DataObject).map_err(|error| error.to_string())?;
            SHDoDragDrop(None, &data, None, DROPEFFECT_COPY | DROPEFFECT_LINK).map_err(|error| error.to_string())
        })();
        for item in items {
            ILFree(Some(item));
        }
        outcome.map(|_| ()).map_err(|error| format!("Cannot drag the files: {error}"))
    }
}

/// Drags text out of the app (into an editor, a chat, a browser field).
/// Must run on the UI thread, while the mouse button is held.
pub fn start_drag_text(text: &str) -> Result<(), String> {
    let _apartment = enter_com();
    // SAFETY: the data object takes ownership of the global block (release = true).
    unsafe {
        let data: IDataObject = SHCreateDataObject(None, None, None).map_err(|error| error.to_string())?;
        let memory = global_from(&utf16_bytes(text))?;
        let format = FORMATETC {
            cfFormat: CF_UNICODETEXT.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: memory },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        };
        if let Err(error) = data.SetData(&format, &medium, true) {
            let _ = GlobalFree(Some(memory));
            return Err(format!("Cannot drag the text: {error}"));
        }
        SHDoDragDrop(None, &data, None, DROPEFFECT_COPY)
            .map(|_| ())
            .map_err(|error| format!("Cannot drag the text: {error}"))
    }
}

/// Encrypts bytes for the current Windows user (DPAPI).
pub fn protect_for_user(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, true)
}

/// Decrypts bytes written by `protect_for_user`.
pub fn unprotect_for_user(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, false)
}

fn crypt(bytes: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` borrows `bytes` for the call; `output` is allocated by
    // DPAPI, copied, then released with LocalFree.
    unsafe {
        let outcome = if protect {
            CryptProtectData(&input, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        } else {
            CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        };
        outcome.map_err(|error| format!("Cannot {} the data: {error}", if protect { "protect" } else { "read" }))?;
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut core::ffi::c_void)));
        Ok(result)
    }
}
