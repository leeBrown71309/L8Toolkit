//! Pixel conversion and PNG encoding for icons, thumbnails and backdrops.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// PNG data URL the web layer can use directly in `src` or `url()`.
pub fn png_data_url(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
    let png = encode_png(rgba, width, height)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

/// Box-filter downscale by an integer factor. Backdrops are blurred heavily
/// anyway, so a quarter-size image looks identical and encodes ~16x faster.
pub fn downscale_rgba(rgba: &[u8], width: u32, height: u32, factor: u32) -> (Vec<u8>, u32, u32) {
    let factor = factor.max(1);
    let (small_width, small_height) = ((width / factor).max(1), (height / factor).max(1));
    let mut output = vec![0u8; (small_width * small_height * 4) as usize];
    for small_y in 0..small_height {
        for small_x in 0..small_width {
            let mut sums = [0u32; 4];
            let mut count = 0u32;
            for y in (small_y * factor)..((small_y + 1) * factor).min(height) {
                for x in (small_x * factor)..((small_x + 1) * factor).min(width) {
                    let offset = ((y * width + x) * 4) as usize;
                    for channel in 0..4 {
                        sums[channel] += rgba[offset + channel] as u32;
                    }
                    count += 1;
                }
            }
            let target = ((small_y * small_width + small_x) * 4) as usize;
            for channel in 0..4 {
                output[target + channel] = (sums[channel] / count.max(1)) as u8;
            }
        }
    }
    (output, small_width, small_height)
}

/// Converts Windows' premultiplied BGRA into straight RGBA, in place.
/// Bitmaps without any alpha (old 24-bit icons) are made fully opaque.
pub fn bgra_premultiplied_to_rgba(pixels: &mut [u8]) {
    let has_alpha = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let (blue, green, red, alpha) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        if !has_alpha {
            pixel.copy_from_slice(&[red, green, blue, u8::MAX]);
            continue;
        }
        if alpha == 0 {
            pixel.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let unpremultiply = |channel: u8| ((channel as u32 * 255 + alpha as u32 / 2) / alpha as u32).min(255) as u8;
        pixel.copy_from_slice(&[unpremultiply(red), unpremultiply(green), unpremultiply(blue), alpha]);
    }
}

pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| format!("PNG header: {error}"))?;
        writer.write_image_data(rgba).map_err(|error| format!("PNG data: {error}"))?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpremultiplies_and_swaps_channels() {
        // Half-transparent pure red, premultiplied: B=0 G=0 R=128 A=128.
        let mut pixels = vec![0, 0, 128, 128, 0, 0, 0, 0];
        bgra_premultiplied_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![255, 0, 0, 128, 0, 0, 0, 0]);
    }

    #[test]
    fn opaque_bitmaps_without_alpha_stay_visible() {
        let mut pixels = vec![10, 20, 30, 0];
        bgra_premultiplied_to_rgba(&mut pixels);
        assert_eq!(pixels, vec![30, 20, 10, 255]);
    }

    #[test]
    fn downscale_averages_blocks() {
        // 2x2 image: black, white, white, black -> one mid-grey pixel.
        let pixels = [0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255];
        let (small, width, height) = downscale_rgba(&pixels, 2, 2, 2);
        assert_eq!((width, height), (1, 1));
        assert_eq!(small, vec![127, 127, 127, 255]);
    }

    #[test]
    fn encodes_a_valid_png_signature() {
        let png = encode_png(&[255, 0, 0, 255], 1, 1).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
