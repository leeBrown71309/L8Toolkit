//! A native drop target laid over the notch while files are dragged to it.
//!
//! The notch's WebView2 cannot take file drops: its top window belongs to the
//! WebView2 process, which registers its own drop target there (refusing
//! external drops, as Tauri asks), and Tauri's target on our side of the
//! window tree is never reached. So the notch gets a plain Win32 window of
//! its own: almost fully transparent, shown over the notch only while a drag
//! comes in, with an OLE drop target that hands the file paths to Rust.
//!
//! The window lives on a thread of its own, initialised for OLE, with its own
//! message loop: drop calls arrive there.

use std::cell::Cell;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread;

use windows::core::w;
use windows_core::{implement, Ref};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINTL, WPARAM};
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium, CF_HDROP, DROPEFFECT,
    DROPEFFECT_COPY, DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, SetLayeredWindowAttributes,
    SetWindowPos, ShowWindow, HWND_TOPMOST, LWA_ALPHA, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_HIDE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::platform::{DropZoneEvent, DropZoneHandler};

/// Lowest opacity that still takes the mouse: invisible to the eye.
const NEARLY_TRANSPARENT: u8 = 1;

static WINDOW: AtomicIsize = AtomicIsize::new(0);

fn window() -> Option<HWND> {
    let handle = WINDOW.load(Ordering::SeqCst);
    (handle != 0).then_some(HWND(handle as *mut _))
}

/// Creates the (hidden) drop window; `handler` is called on its thread for
/// every drag that brings files over it.
pub fn start_drop_zone(handler: DropZoneHandler) -> Result<(), String> {
    thread::Builder::new()
        .name("notch-drop-zone".into())
        .spawn(move || {
            // SAFETY: the window, its drop target and the OLE apartment all
            // live on this thread, which pumps their messages until the app ends.
            unsafe {
                if let Err(error) = OleInitialize(None) {
                    log::warn!("Files cannot be dropped on the notch: OLE is unavailable ({error})");
                    return;
                }
                let instance = GetModuleHandleW(None).unwrap_or_default();
                let class = WNDCLASSW {
                    lpfnWndProc: Some(window_procedure),
                    hInstance: instance.into(),
                    lpszClassName: w!("L8NotchDropZone"),
                    ..Default::default()
                };
                RegisterClassW(&class);
                let created = CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                    w!("L8NotchDropZone"),
                    None,
                    WS_POPUP,
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    Some(instance.into()),
                    None,
                );
                let hwnd = match created {
                    Ok(hwnd) => hwnd,
                    Err(error) => {
                        log::warn!("Files cannot be dropped on the notch: no drop window ({error})");
                        return;
                    }
                };
                if let Err(error) = SetLayeredWindowAttributes(hwnd, COLORREF(0), NEARLY_TRANSPARENT, LWA_ALPHA) {
                    log::warn!("Notch drop window: cannot set its transparency ({error})");
                }
                let target: IDropTarget = DropZoneTarget { handler, has_files: Cell::new(false) }.into();
                if let Err(error) = RegisterDragDrop(hwnd, &target) {
                    log::warn!("Files cannot be dropped on the notch: {error}");
                    return;
                }
                WINDOW.store(hwnd.0 as isize, Ordering::SeqCst);

                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    DispatchMessageW(&message);
                }
            }
        })
        .map(|_| ())
        .map_err(|error| format!("Cannot start the notch drop zone: {error}"))
}

/// Lays the drop window over `(x, y, width, height)`, above every other window.
pub fn show_drop_zone(x: i32, y: i32, width: i32, height: i32) {
    if let Some(hwnd) = window() {
        // SAFETY: our own window; showing it never takes the focus.
        let result = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW) };
        if let Err(error) = result {
            log::debug!("Notch drop window: cannot show it ({error})");
        }
    }
}

/// Puts the drop window back on top (the notch window rises when it opens).
pub fn raise_drop_zone() {
    if let Some(hwnd) = window() {
        // SAFETY: our own window; only its z-order changes.
        let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE) };
    }
}

pub fn hide_drop_zone() {
    if let Some(hwnd) = window() {
        // SAFETY: our own window; the return value is its previous visibility.
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }
}

unsafe extern "system" fn window_procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: default handling for a window of ours.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

#[implement(IDropTarget)]
struct DropZoneTarget {
    handler: DropZoneHandler,
    /// The drag in progress carries files (other drags are refused).
    has_files: Cell<bool>,
}

impl IDropTarget_Impl for DropZoneTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let paths = data.as_ref().map(read_paths).unwrap_or_default();
        self.has_files.set(!paths.is_empty());
        // SAFETY: OLE passes a valid pointer for the effect.
        unsafe { *effect = if paths.is_empty() { DROPEFFECT_NONE } else { DROPEFFECT_COPY } };
        if !paths.is_empty() {
            (self.handler)(DropZoneEvent::Enter);
        }
        Ok(())
    }

    fn DragOver(&self, _keys: MODIFIERKEYS_FLAGS, _point: &POINTL, effect: *mut DROPEFFECT) -> windows::core::Result<()> {
        // SAFETY: OLE passes a valid pointer for the effect.
        unsafe { *effect = if self.has_files.get() { DROPEFFECT_COPY } else { DROPEFFECT_NONE } };
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if self.has_files.replace(false) {
            (self.handler)(DropZoneEvent::Leave);
        }
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        _point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let paths = data.as_ref().map(read_paths).unwrap_or_default();
        self.has_files.set(false);
        // SAFETY: OLE passes a valid pointer for the effect.
        unsafe { *effect = if paths.is_empty() { DROPEFFECT_NONE } else { DROPEFFECT_COPY } };
        (self.handler)(DropZoneEvent::Drop(paths));
        Ok(())
    }
}

/// The file paths of a drag (CF_HDROP), empty when it carries none.
fn read_paths(data: &IDataObject) -> Vec<String> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    // SAFETY: the medium is released once the paths are copied out.
    unsafe {
        let Ok(mut medium) = data.GetData(&format) else {
            return Vec::new();
        };
        let drop = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let paths = (0..count)
            .map(|index| {
                let length = DragQueryFileW(drop, index, None) as usize;
                let mut buffer = vec![0u16; length + 1];
                DragQueryFileW(drop, index, Some(&mut buffer));
                OsString::from_wide(&buffer[..length]).to_string_lossy().into_owned()
            })
            .collect();
        ReleaseStgMedium(&mut medium);
        paths
    }
}
