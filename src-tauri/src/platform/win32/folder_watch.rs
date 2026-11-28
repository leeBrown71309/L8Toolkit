//! Change notifications for folder trees, so indexes refresh when something
//! is added instead of on a timer. The thread sleeps in the kernel until a
//! name changes somewhere under a watched folder: no polling, no CPU at rest.

use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::thread;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME,
};
use windows::Win32::System::Threading::{WaitForMultipleObjects, INFINITE};

use crate::platform::FolderChangeHandler;

/// `WaitForMultipleObjects` takes at most this many handles.
const MAX_WATCHED_FOLDERS: usize = 64;

/// Calls `handler(index)` from a thread of its own whenever a file or folder
/// is created, renamed or deleted under `folders[index]` (subfolders included).
/// Folders that cannot be watched (missing, no access) are skipped.
pub fn watch_folders(folders: Vec<PathBuf>, handler: FolderChangeHandler) -> Result<(), String> {
    let mut watched = Vec::new();
    for (index, folder) in folders.iter().enumerate().take(MAX_WATCHED_FOLDERS) {
        let wide: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `wide` is a NUL-terminated path that outlives the call.
        let handle = unsafe {
            FindFirstChangeNotificationW(
                PCWSTR(wide.as_ptr()),
                true,
                FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME,
            )
        };
        match handle {
            Ok(handle) => watched.push((index, handle.0 as isize)),
            Err(error) => log::debug!("Cannot watch {}: {error}", folder.display()),
        }
    }
    if watched.is_empty() {
        return Err("No folder could be watched for changes".to_string());
    }

    thread::Builder::new()
        .name("folder-watch".into())
        .spawn(move || {
            let handles: Vec<HANDLE> = watched.iter().map(|(_, handle)| HANDLE(*handle as *mut _)).collect();
            loop {
                // SAFETY: the handles stay open for the life of this thread.
                let signaled = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };
                let position = signaled.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
                let Some(&(index, _)) = watched.get(position) else {
                    log::warn!("Folder watch stopped: wait failed ({:#x})", signaled.0);
                    break;
                };
                handler(index);
                // SAFETY: re-arms the notification that just fired.
                if let Err(error) = unsafe { FindNextChangeNotification(handles[position]) } {
                    log::warn!("Folder watch stopped: {error}");
                    break;
                }
            }
            for handle in handles {
                // SAFETY: each handle is closed once, when the watch ends.
                let _ = unsafe { FindCloseChangeNotification(handle) };
            }
        })
        .map(|_| ())
        .map_err(|error| format!("Cannot start the folder watch: {error}"))
}
