use std::ffi::c_void;
use std::mem::size_of;

use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
    SIIGBF_RESIZETOFIT,
};

/// Shell icon of any parsing name (file path, folder or `shell:AppsFolder\...`
/// id) as top-down, non-premultiplied RGBA pixels.
/// The calling thread must have entered a COM apartment.
pub fn extract_icon_rgba(target: &str, size: i32) -> Result<(Vec<u8>, u32, u32), String> {
    render_shell_image(target, size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
}

/// Thumbnail of an image file, fitted inside `size` x `size`.
/// The calling thread must have entered a COM apartment.
pub fn extract_thumbnail_rgba(path: &str, size: i32) -> Result<(Vec<u8>, u32, u32), String> {
    render_shell_image(path, size, SIIGBF_RESIZETOFIT | SIIGBF_BIGGERSIZEOK)
}

fn render_shell_image(target: &str, size: i32, flags: SIIGBF) -> Result<(Vec<u8>, u32, u32), String> {
    // SAFETY: COM calls on an initialised apartment; the bitmap is always freed.
    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(target), None)
            .map_err(|error| format!("Cannot resolve '{target}': {error}"))?;
        let bitmap = factory
            .GetImage(SIZE { cx: size, cy: size }, flags)
            .map_err(|error| format!("Cannot render an image for '{target}': {error}"))?;
        let pixels = read_bitmap_pixels(bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        pixels
    }
}

/// Reads any GDI bitmap as straight RGBA (top-down rows).
pub(super) unsafe fn read_bitmap_pixels(bitmap: HBITMAP) -> Result<(Vec<u8>, u32, u32), String> {
    let mut header = BITMAP::default();
    let written = GetObjectW(
        HGDIOBJ(bitmap.0),
        size_of::<BITMAP>() as i32,
        Some(&mut header as *mut BITMAP as *mut c_void),
    );
    if written == 0 || header.bmWidth <= 0 || header.bmHeight == 0 {
        return Err("Bitmap has no readable dimensions".to_string());
    }

    let width = header.bmWidth;
    let height = header.bmHeight.abs();
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative height requests a top-down DIB, matching PNG row order.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let device_context = GetDC(None);
    let lines = GetDIBits(
        device_context,
        bitmap,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as *mut c_void),
        &mut info,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, device_context);

    if lines == 0 {
        return Err("Cannot read bitmap pixels".to_string());
    }

    crate::icons::encoding::bgra_premultiplied_to_rgba(&mut pixels);
    Ok((pixels, width as u32, height as u32))
}
