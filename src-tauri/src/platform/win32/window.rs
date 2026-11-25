use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetClassNameW, GetCursorPos, GetDesktopWindow, GetForegroundWindow,
    GetShellWindow, GetWindowRect, GetWindowThreadProcessId, IsWindow, IsZoomed, SetForegroundWindow,
};

use crate::platform::ScreenRect;

/// Desktop wallpaper hosts report a full-screen rect but are not "fullscreen apps".
const DESKTOP_WINDOW_CLASSES: [&str; 2] = ["WorkerW", "Progman"];

fn to_hwnd(handle: isize) -> HWND {
    HWND(handle as *mut c_void)
}

pub fn foreground_window() -> isize {
    // SAFETY: read-only query.
    unsafe { GetForegroundWindow().0 as isize }
}

/// Gives focus back to the window that was active before the notch opened.
pub fn restore_foreground(handle: isize) {
    if handle == 0 {
        return;
    }
    let hwnd = to_hwnd(handle);
    // SAFETY: the handle is validated with IsWindow before use.
    unsafe {
        if IsWindow(Some(hwnd)).as_bool() {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Brings one of our windows to the foreground even when Windows' focus-steal
/// prevention would refuse it (e.g. when opened by hovering the notch).
/// Must run on the thread that owns the window.
pub fn force_foreground(handle: isize) {
    if handle == 0 {
        return;
    }
    let hwnd = to_hwnd(handle);
    // SAFETY: thread input is detached again before returning.
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            return;
        }
        let foreground_thread = GetWindowThreadProcessId(foreground, None);
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(foreground_thread, current_thread, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(foreground_thread, current_thread, false);
        }
    }
}

pub fn cursor_position() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    // SAFETY: writes into a local POINT.
    unsafe { GetCursorPos(&mut point) }.ok().map(|_| (point.x, point.y))
}

/// Returns the monitor rect covered by the foreground window when that window
/// is fullscreen (games, videos, presentations), so the notch can step aside.
pub fn fullscreen_foreground_rect(own_window: isize) -> Option<ScreenRect> {
    // SAFETY: read-only window queries on the current foreground window.
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_invalid()
            || foreground.0 as isize == own_window
            || foreground == GetDesktopWindow()
            || foreground == GetShellWindow()
            || is_desktop_host(foreground)
        {
            return None;
        }

        // A maximized window can cover the whole monitor when the taskbar
        // auto-hides; that is not a fullscreen app.
        if IsZoomed(foreground).as_bool() {
            return None;
        }
        let mut window_rect = RECT::default();
        GetWindowRect(foreground, &mut window_rect).ok()?;

        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let screen = info.rcMonitor;
        let covers_monitor = window_rect.left <= screen.left
            && window_rect.top <= screen.top
            && window_rect.right >= screen.right
            && window_rect.bottom >= screen.bottom;
        covers_monitor.then_some((screen.left, screen.top, screen.right, screen.bottom))
    }
}

unsafe fn is_desktop_host(hwnd: HWND) -> bool {
    let mut buffer = [0u16; 64];
    let length = GetClassNameW(hwnd, &mut buffer);
    if length <= 0 {
        return false;
    }
    let class_name = String::from_utf16_lossy(&buffer[..length as usize]);
    DESKTOP_WINDOW_CLASSES.contains(&class_name.as_str())
}
