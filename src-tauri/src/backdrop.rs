//! Images behind the glass materials.
//!
//! Windows' own Mica/Acrylic backdrops ignore custom window shapes (they
//! paint the whole rectangular canvas), so the notch renders its materials
//! itself from two sources:
//! * Mica: the desktop wallpaper, blurred and tinted, exactly what real Mica
//!   samples.
//! * Acrylic: a snapshot of the screen behind the notch, taken just before it
//!   opens, blurred in the web layer. Mica falls back to it when the
//!   wallpaper cannot be read (solid color, slideshow being swapped...).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::icons::encoding::{downscale_rgba, png_data_url};
use crate::platform::{self, ScreenRect};

/// Wallpaper thumbnail size: plenty once blurred by 70px.
const WALLPAPER_THUMBNAIL_SIZE: i32 = 720;
const SNAPSHOT_DOWNSCALE_FACTOR: u32 = 4;
const KNOWN_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "webp", "gif", "jxr", "heic"];

/// Cheap to clone: shared with the blocking worker that renders thumbnails.
#[derive(Clone)]
pub struct WallpaperCache {
    /// Where an extension-less wallpaper gets a readable copy.
    cache_directory: PathBuf,
    /// (path and modification time, data URL)
    entry: Arc<Mutex<Option<(String, String)>>>,
}

impl WallpaperCache {
    pub fn new(cache_directory: PathBuf) -> Self {
        Self { cache_directory, entry: Arc::default() }
    }

    /// Blocking (file copy, shell thumbnail): call from a background thread.
    pub fn data_url(&self) -> Result<Option<String>, String> {
        let Some(path) = platform::desktop_wallpaper_path() else { return Ok(None) };
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let key = format!("{path}|{modified:?}");

        let mut entry = self.entry.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached_key, data_url)) = entry.as_ref() {
            if *cached_key == key {
                return Ok(Some(data_url.clone()));
            }
        }
        let readable = self.readable_copy(Path::new(&path))?;
        let _apartment = platform::enter_com();
        let (pixels, width, height) =
            platform::extract_thumbnail_rgba(&readable.to_string_lossy(), WALLPAPER_THUMBNAIL_SIZE)?;
        let data_url = png_data_url(&pixels, width, height)?;
        *entry = Some((key, data_url.clone()));
        Ok(Some(data_url))
    }

    /// Windows keeps the active wallpaper as `TranscodedWallpaper`, without
    /// an extension, and the shell needs one to pick an image decoder.
    fn readable_copy(&self, path: &Path) -> Result<PathBuf, String> {
        let has_image_extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_lowercase())
            .is_some_and(|extension| KNOWN_IMAGE_EXTENSIONS.contains(&extension.as_str()));
        if has_image_extension {
            return Ok(path.to_path_buf());
        }
        let header = read_header(path)?;
        let extension = image_extension_from_signature(&header)
            .ok_or_else(|| format!("{} is not a recognised image", path.display()))?;
        fs::create_dir_all(&self.cache_directory)
            .map_err(|error| format!("Cannot create {}: {error}", self.cache_directory.display()))?;
        let copy = self.cache_directory.join(format!("wallpaper.{extension}"));
        fs::copy(path, &copy).map_err(|error| format!("Cannot copy the wallpaper: {error}"))?;
        Ok(copy)
    }
}

fn read_header(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut header = vec![0u8; 12];
    let mut file = fs::File::open(path).map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    let read = file.read(&mut header).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    header.truncate(read);
    Ok(header)
}

/// Identifies common image formats from their first bytes.
pub fn image_extension_from_signature(header: &[u8]) -> Option<&'static str> {
    match header {
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [b'B', b'M', ..] => Some("bmp"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("webp"),
        _ => None,
    }
}

/// What is on screen behind the notch, small and ready to blur.
pub fn snapshot_data_url(rect: ScreenRect) -> Result<String, String> {
    let (pixels, width, height) = platform::capture_screen_rgba(rect)?;
    let (small, small_width, small_height) = downscale_rgba(&pixels, width, height, SNAPSHOT_DOWNSCALE_FACTOR);
    png_data_url(&small, small_width, small_height)
}

#[cfg(test)]
mod tests {
    use super::image_extension_from_signature;

    #[test]
    fn recognises_image_signatures() {
        assert_eq!(image_extension_from_signature(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_extension_from_signature(b"\x89PNG\r\n\x1a\n"), Some("png"));
        assert_eq!(image_extension_from_signature(b"RIFF\0\0\0\0WEBPVP8"), Some("webp"));
        assert_eq!(image_extension_from_signature(b"hello"), None);
    }
}
