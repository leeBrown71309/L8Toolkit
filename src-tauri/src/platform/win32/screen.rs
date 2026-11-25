use std::ffi::c_void;

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    CAPTUREBLT, HGDIOBJ, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};

use super::icons::read_bitmap_pixels;
use crate::platform::ScreenRect;

const MAX_PATH_LENGTH: usize = 260;

/// Copies a rectangle of the screen (physical pixels) as RGBA.
pub fn capture_screen_rgba(rect: ScreenRect) -> Result<(Vec<u8>, u32, u32), String> {
    let (left, top, right, bottom) = rect;
    let (width, height) = (right - left, bottom - top);
    if width <= 0 || height <= 0 {
        return Err("Empty capture area".to_string());
    }
    // SAFETY: every GDI object created here is selected out and released.
    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let previous = SelectObject(memory, HGDIOBJ(bitmap.0));
        // CAPTUREBLT includes layered windows, like what the user actually sees.
        let copied = BitBlt(memory, 0, 0, width, height, Some(screen), left, top, ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0));
        SelectObject(memory, previous);
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);

        let pixels = copied
            .map_err(|error| format!("Cannot capture the screen: {error}"))
            .and_then(|_| read_bitmap_pixels(bitmap));
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        pixels
    }
}

/// Path of the current desktop wallpaper image, if any.
pub fn desktop_wallpaper_path() -> Option<String> {
    let mut buffer = [0u16; MAX_PATH_LENGTH];
    // SAFETY: the buffer length is passed along with the pointer.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buffer.len() as u32,
            Some(buffer.as_mut_ptr() as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    read.ok()?;
    let length = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..length]);
    (!path.is_empty() && std::path::Path::new(&path).exists()).then_some(path)
}
