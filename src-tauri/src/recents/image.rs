//! Clipboard images: decoding the "PNG" and CF_DIB formats to RGBA, writing
//! a bitmap back for the clipboard, and small thumbnails for the notch.

use std::io::Cursor;

use crate::icons::encoding::downscale_rgba;

const BITMAP_INFO_HEADER_SIZE: usize = 40;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
/// Biggest image kept (a 16K screenshot); larger ones are skipped.
const MAX_PIXELS: u64 = 16_384 * 16_384;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn check_size(width: u32, height: u32) -> Result<(), String> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!("Unsupported image size {width}x{height}"));
    }
    Ok(())
}

pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|error| format!("PNG: {error}"))?;
    let size = reader.output_buffer_size().ok_or("PNG: image too large")?;
    let mut buffer = vec![0u8; size];
    let info = reader.next_frame(&mut buffer).map_err(|error| format!("PNG: {error}"))?;
    check_size(info.width, info.height)?;
    buffer.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer.as_chunks::<3>().0.iter().flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX]).collect(),
        png::ColorType::Grayscale => buffer.iter().flat_map(|&gray| [gray, gray, gray, u8::MAX]).collect(),
        png::ColorType::GrayscaleAlpha => buffer.as_chunks::<2>().0.iter().flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]]).collect(),
        png::ColorType::Indexed => return Err("PNG: palette not expanded".to_string()),
    };
    Ok(RgbaImage { pixels, width: info.width, height: info.height })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
        .ok_or_else(|| "Bitmap: truncated".to_string())
}

/// CF_DIB: a BITMAPINFOHEADER (or V4/V5) followed by the pixels. Handles the
/// 24 and 32-bit images apps put on the clipboard (screenshots, browsers).
pub fn decode_dib(bytes: &[u8]) -> Result<RgbaImage, String> {
    let header_size = read_u32(bytes, 0)? as usize;
    if header_size < BITMAP_INFO_HEADER_SIZE {
        return Err("Bitmap: unsupported header".to_string());
    }
    let width = read_u32(bytes, 4)? as i32;
    let raw_height = read_u32(bytes, 8)? as i32;
    let bit_count = u16::from_le_bytes([bytes[14], bytes[15]]);
    let compression = read_u32(bytes, 16)?;
    let colors_used = read_u32(bytes, 32)? as usize;
    if width <= 0 || raw_height == 0 || !(bit_count == 24 || bit_count == 32) {
        return Err(format!("Bitmap: unsupported format ({bit_count} bits)"));
    }
    if compression != BI_RGB && !(compression == BI_BITFIELDS && bit_count == 32) {
        return Err("Bitmap: compressed images are not supported".to_string());
    }
    let (width, height) = (width as u32, raw_height.unsigned_abs());
    check_size(width, height)?;
    // A plain 40-byte header is followed by the three color masks (BITFIELDS).
    let masks = if compression == BI_BITFIELDS && header_size == BITMAP_INFO_HEADER_SIZE { 12 } else { 0 };
    let pixels_offset = header_size + masks + colors_used * 4;
    let bytes_per_pixel = usize::from(bit_count / 8);
    let stride = (width as usize * bytes_per_pixel).div_ceil(4) * 4;
    let needed = pixels_offset + stride * height as usize;
    if bytes.len() < needed {
        return Err("Bitmap: truncated pixels".to_string());
    }
    let bottom_up = raw_height > 0;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height as usize {
        let source_row = if bottom_up { height as usize - 1 - row } else { row };
        let start = pixels_offset + source_row * stride;
        for pixel in bytes[start..start + width as usize * bytes_per_pixel].chunks_exact(bytes_per_pixel) {
            let alpha = if bytes_per_pixel == 4 { pixel[3] } else { u8::MAX };
            pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], alpha]);
        }
    }
    // Most 32-bit clipboard bitmaps leave alpha at 0: they are opaque.
    if bytes_per_pixel == 4 && pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0) {
        pixels.as_chunks_mut::<4>().0.iter_mut().for_each(|pixel| pixel[3] = u8::MAX);
    }
    Ok(RgbaImage { pixels, width, height })
}

/// A 32-bit bottom-up CF_DIB, the most widely understood clipboard bitmap.
pub fn encode_dib(image: &RgbaImage) -> Vec<u8> {
    let row_bytes = image.width as usize * 4;
    let mut bytes = Vec::with_capacity(BITMAP_INFO_HEADER_SIZE + row_bytes * image.height as usize);
    bytes.extend_from_slice(&(BITMAP_INFO_HEADER_SIZE as u32).to_le_bytes());
    bytes.extend_from_slice(&(image.width as i32).to_le_bytes());
    bytes.extend_from_slice(&(image.height as i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&BI_RGB.to_le_bytes());
    bytes.extend_from_slice(&((row_bytes * image.height as usize) as u32).to_le_bytes());
    bytes.extend_from_slice(&[0u8; 16]);
    for row in (0..image.height as usize).rev() {
        for pixel in image.pixels[row * row_bytes..(row + 1) * row_bytes].as_chunks::<4>().0 {
            bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    bytes
}

/// A copy whose longest edge is at most about `max_edge` pixels.
pub fn thumbnail(image: &RgbaImage, max_edge: u32) -> RgbaImage {
    let longest = image.width.max(image.height);
    let factor = longest.div_ceil(max_edge.max(1)).max(1);
    if factor == 1 {
        return image.clone();
    }
    let (pixels, width, height) = downscale_rgba(&image.pixels, image.width, image.height, factor);
    RgbaImage { pixels, width, height }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::encoding::encode_png;

    fn sample() -> RgbaImage {
        // 2x2: red, green / blue, half-transparent white.
        RgbaImage { pixels: vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128], width: 2, height: 2 }
    }

    #[test]
    fn dib_round_trips() {
        let image = sample();
        assert_eq!(decode_dib(&encode_dib(&image)).unwrap(), image);
    }

    #[test]
    fn png_round_trips() {
        let image = sample();
        let png = encode_png(&image.pixels, image.width, image.height).unwrap();
        assert_eq!(decode_png(&png).unwrap(), image);
    }

    #[test]
    fn opaque_bitmaps_without_alpha_stay_visible() {
        let mut dib = encode_dib(&sample());
        // Zero every alpha byte, as screenshot tools do.
        for pixel in dib[BITMAP_INFO_HEADER_SIZE..].as_chunks_mut::<4>().0 {
            pixel[3] = 0;
        }
        let decoded = decode_dib(&dib).unwrap();
        assert!(decoded.pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == u8::MAX));
    }

    #[test]
    fn twenty_four_bit_rows_are_padded() {
        // 1x2, 24 bits: each 3-byte row is padded to 4 bytes. Bottom-up: blue row first.
        let mut dib = vec![0u8; 40];
        dib[0] = 40;
        dib[4] = 1;
        dib[8] = 2;
        dib[12] = 1;
        dib[14] = 24;
        dib.extend_from_slice(&[255, 0, 0, 0, 0, 0, 255, 0]);
        let decoded = decode_dib(&dib).unwrap();
        assert_eq!(decoded.pixels, vec![255, 0, 0, 255, 0, 0, 255, 255]);
    }

    #[test]
    fn thumbnails_shrink_the_longest_edge() {
        let image = RgbaImage { pixels: vec![0; 1000 * 500 * 4], width: 1000, height: 500 };
        let small = thumbnail(&image, 300);
        assert!(small.width <= 300 && small.height <= 150, "{}x{}", small.width, small.height);
    }
}
