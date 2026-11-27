//! Recents: the last texts, images, screenshots and files copied, plus what
//! was dropped on the notch (the shelf). Each item can be copied again,
//! dragged into another app or opened.
//!
//! - `list`: the list logic (pure, tested).
//! - `image`: clipboard image formats and thumbnails (pure, tested).
//! - `secret`: spots texts that look like passwords or keys (pure, tested).
//! - this module: the clipboard listener, the files on disk, the commands.
//!
//! Stored in the app data folder: `recents/recents.json` and `recents/images/`.
//! Sensitive texts (copied from a password manager, or looking like a password
//! or a key) are encrypted for the Windows user (DPAPI) before being written,
//! and never sent to the web layer in clear.

mod image;
mod list;
mod secret;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub use list::{RecentKind, RecentOrigin};
use list::{fingerprint, RecentItem, RecentList, StoredImage};

use crate::i18n::{tr, Text};
use crate::icons::encoding::encode_png;
use crate::json_file;
use crate::notch;
use crate::platform::{self, ClipboardChange, ClipboardContent, ShellVerb};
use crate::settings::{RecentsSettings, SettingsStore};

pub const RECENTS_CHANGED_EVENT: &str = "recents:changed";
pub const RECENTS_SCREENSHOT_EVENT: &str = "recents:screenshot";
const STORE_FILE: &str = "recents.json";
const IMAGES_FOLDER: &str = "images";
const THUMBNAIL_EDGE: u32 = 360;
/// Thumbnails of copied or dropped files: enough for a card, quick to render.
const FILE_PREVIEW_EDGE: i32 = 256;
/// Enough text for three lines of preview; the full text stays in Rust.
const PREVIEW_CHARS: usize = 400;
/// How long the notch stays up to show a new screenshot (the web layer peeks 4 s).
const SCREENSHOT_PEEK_HOLD: Duration = Duration::from_millis(4600);
/// Apps whose images are screen captures.
const SCREENSHOT_APPS: &[&str] = &[
    "screenclippinghost.exe", "snippingtool.exe", "screensketch.exe", "sharex.exe", "greenshot.exe",
    "lightshot.exe", "flameshot.exe", "snagit32.exe", "snagiteditor.exe",
];

/// What the Recents tab draws for one item.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentView {
    pub id: String,
    pub kind: RecentKind,
    pub origin: RecentOrigin,
    pub created_at_ms: u64,
    /// The beginning of a text; none for sensitive items.
    pub preview: Option<String>,
    pub text_length: usize,
    pub files: Vec<RecentFile>,
    /// Small PNG data URL: the image, or the first file's thumbnail when
    /// Windows has one (photos, videos, some documents).
    pub thumbnail: Option<String>,
    pub image_size: Option<[u32; 2]>,
    pub sensitive: bool,
    /// The app that copied, readable ("Bitwarden").
    pub source_app: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    /// False once the file was moved or deleted.
    pub exists: bool,
}

pub struct RecentsService {
    directory: PathBuf,
    list: Mutex<RecentList>,
    thumbnails: Mutex<HashMap<String, String>>,
    /// File thumbnails by path and modification time; `None`: Windows has none.
    file_previews: Mutex<HashMap<String, Option<String>>>,
    counter: AtomicU64,
}

impl RecentsService {
    /// Reads the saved list; sensitive texts are decrypted, items whose image
    /// went missing are dropped. Texts saved in clear that look like secrets
    /// (kept by an earlier version) are marked sensitive and saved encrypted.
    pub fn load(directory: PathBuf) -> Self {
        let stored: Vec<RecentItem> = json_file::read_or_default(&directory.join(STORE_FILE));
        let images = directory.join(IMAGES_FOLDER);
        let mut newly_masked = false;
        let items = stored
            .into_iter()
            .filter_map(|mut item| {
                if let Some(protected) = item.protected_text.take() {
                    item.text = Some(reveal(&protected).map_err(|error| log::warn!("Recents: {error}")).ok()?);
                }
                if !item.sensitive && item.text.as_deref().is_some_and(secret::looks_like_secret) {
                    item.sensitive = true;
                    newly_masked = true;
                }
                let image_present = item.image.as_ref().is_none_or(|image| images.join(&image.file).exists());
                image_present.then_some(item)
            })
            .collect();
        let service = RecentsService {
            directory,
            list: Mutex::new(RecentList::new(items)),
            thumbnails: Mutex::new(HashMap::new()),
            file_previews: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(0),
        };
        if newly_masked {
            service.save(&service.lock());
        }
        service
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, RecentList> {
        self.list.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn images_folder(&self) -> PathBuf {
        self.directory.join(IMAGES_FOLDER)
    }

    fn next_id(&self) -> String {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("{:x}{count:x}", now_ms())
    }

    pub fn views(&self) -> Vec<RecentView> {
        let items = self.lock().items().to_vec();
        items.iter().map(|item| self.view(item)).collect()
    }

    fn view(&self, item: &RecentItem) -> RecentView {
        let text = item.text.as_deref().unwrap_or_default();
        RecentView {
            id: item.id.clone(),
            kind: item.kind,
            origin: item.origin,
            created_at_ms: item.created_at_ms,
            preview: (!item.sensitive && item.kind == RecentKind::Text).then(|| text.chars().take(PREVIEW_CHARS).collect()),
            text_length: text.chars().count(),
            files: item
                .files
                .iter()
                .map(|path| RecentFile {
                    path: path.clone(),
                    name: Path::new(path).file_name().map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned()),
                    exists: Path::new(path).exists(),
                })
                .collect(),
            thumbnail: match &item.image {
                Some(image) => self.thumbnail_url(&image.thumbnail),
                None => item.files.first().and_then(|path| self.file_preview(path)),
            },
            image_size: item.image.as_ref().map(|image| [image.width, image.height]),
            sensitive: item.sensitive,
            source_app: item.source_app.as_deref().map(readable_app_name),
        }
    }

    fn thumbnail_url(&self, file: &str) -> Option<String> {
        let mut cache = self.thumbnails.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(url) = cache.get(file) {
            return Some(url.clone());
        }
        let bytes = fs::read(self.images_folder().join(file)).ok()?;
        let url = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
        cache.insert(file.to_string(), url.clone());
        Some(url)
    }

    /// The thumbnail Windows renders for a file, as a PNG data URL, cached
    /// until the file changes. `None` for files with only an icon.
    fn file_preview(&self, path: &str) -> Option<String> {
        let modified = fs::metadata(path).and_then(|metadata| metadata.modified()).ok()?;
        let since_epoch = modified.duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_millis());
        let key = format!("{path}|{since_epoch}");
        let mut cache = self.file_previews.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
        let _apartment = platform::enter_com();
        let preview = platform::extract_file_preview_rgba(path, FILE_PREVIEW_EDGE)
            .and_then(|(pixels, width, height)| encode_png(&pixels, width, height))
            .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)))
            .map_err(|error| log::debug!("Recents: no preview for a file: {error}"))
            .ok();
        cache.insert(key, preview.clone());
        preview
    }

    /// Writes the list; sensitive texts are encrypted first.
    fn save(&self, list: &RecentList) {
        let stored: Vec<RecentItem> = list
            .items()
            .iter()
            .filter_map(|item| {
                let mut stored = item.clone();
                if item.sensitive {
                    let text = stored.text.take()?;
                    stored.protected_text = Some(conceal(&text).map_err(|error| log::warn!("Recents: {error}")).ok()?);
                }
                Some(stored)
            })
            .collect();
        if let Err(error) = json_file::write_atomically(&self.directory.join(STORE_FILE), &stored) {
            log::warn!("Recents: cannot save the list: {error}");
        }
    }

    /// Deletes the image files of items that left the list.
    fn forget_files(&self, removed: &[RecentItem]) {
        let mut cache = self.thumbnails.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        for image in removed.iter().filter_map(|item| item.image.as_ref()) {
            for file in [&image.file, &image.thumbnail] {
                cache.remove(file);
                if let Err(error) = fs::remove_file(self.images_folder().join(file)) {
                    log::debug!("Recents: cannot delete {file}: {error}");
                }
            }
        }
    }

    /// Adds an item, saves, and returns its view.
    fn add(&self, item: RecentItem, limit: usize) -> RecentView {
        let view = self.view(&item);
        let removed = {
            let mut list = self.lock();
            let removed = list.push(item, limit);
            self.save(&list);
            removed
        };
        self.forget_files(&removed);
        view
    }

    /// Applies a list change made by `change`, then saves.
    fn update(&self, change: impl FnOnce(&mut RecentList) -> Vec<RecentItem>) {
        let removed = {
            let mut list = self.lock();
            let removed = change(&mut list);
            self.save(&list);
            removed
        };
        self.forget_files(&removed);
    }

    fn item(&self, id: &str) -> Option<RecentItem> {
        self.lock().get(id).cloned()
    }

    /// Saves an image (full size and thumbnail) under a readable name: it
    /// becomes the file name when the image is dragged into another app.
    fn store_image(&self, image: &image::RgbaImage, label: &str, id: &str) -> Result<StoredImage, String> {
        let folder = self.images_folder();
        fs::create_dir_all(&folder).map_err(|error| format!("Cannot create the Recents folder: {error}"))?;
        let file = format!("{label} {id}.png");
        let thumbnail_file = format!("{id}-thumbnail.png");
        let small = image::thumbnail(image, THUMBNAIL_EDGE);
        fs::write(folder.join(&file), encode_png(&image.pixels, image.width, image.height)?)
            .map_err(|error| format!("Cannot save the image: {error}"))?;
        fs::write(folder.join(&thumbnail_file), encode_png(&small.pixels, small.width, small.height)?)
            .map_err(|error| format!("Cannot save the thumbnail: {error}"))?;
        Ok(StoredImage { file, thumbnail: thumbnail_file, width: image.width, height: image.height })
    }
}

/// Starts listening to the clipboard.
pub fn start(app: &AppHandle) {
    // Secrets found while loading must not stay when the user keeps none.
    if !settings(app).keep_password_manager_items {
        app.state::<RecentsService>().update(|list| list.remove_where(|item| item.sensitive));
    }
    let handle = app.clone();
    if let Err(error) = platform::watch_clipboard(Box::new(move |change| on_clipboard(&handle, change))) {
        log::warn!("Recents is unavailable: {error}");
    }
}

fn settings(app: &AppHandle) -> RecentsSettings {
    app.state::<SettingsStore>().read(|settings| settings.recents.clone())
}

fn on_clipboard(app: &AppHandle, change: ClipboardChange) {
    let settings = settings(app);
    // A browser extension (Bitwarden in Vivaldi…) copies through the browser,
    // which marks nothing: the text itself must then reveal the secret.
    let sensitive = change.sensitive
        || match &change.content {
            ClipboardContent::Text(text) => secret::looks_like_secret(text),
            _ => false,
        };
    if !settings.enabled || (sensitive && !settings.keep_password_manager_items) {
        return;
    }
    let service = app.state::<RecentsService>();
    let limit = settings.limit as usize;
    let now = now_ms();
    let screenshot_app = change.source_app.as_deref().is_none_or(|name| SCREENSHOT_APPS.contains(&name));

    let (kind, origin, text, files, decoded) = match change.content {
        ClipboardContent::Text(text) => (RecentKind::Text, RecentOrigin::Copy, Some(text), Vec::new(), None),
        ClipboardContent::Files(files) => (RecentKind::Files, RecentOrigin::Copy, None, files, None),
        ClipboardContent::Png(bytes) => (RecentKind::Image, image_origin(screenshot_app), None, Vec::new(), Some(image::decode_png(&bytes))),
        ClipboardContent::Dib(bytes) => (RecentKind::Image, image_origin(screenshot_app), None, Vec::new(), Some(image::decode_dib(&bytes))),
    };
    let decoded = match decoded.transpose() {
        Ok(decoded) => decoded,
        Err(error) => {
            log::debug!("Recents: image skipped: {error}");
            return;
        }
    };
    let content_key: Vec<u8> = match (&text, &decoded) {
        (Some(text), _) => text.as_bytes().to_vec(),
        (_, Some(image)) => image.pixels.clone(),
        _ => files.join("\n").into_bytes(),
    };
    let print = fingerprint(kind, &content_key);
    // Copying the same thing again only moves it up.
    let promoted = {
        let mut list = service.lock();
        let promoted = list.promote_fingerprint(print, now);
        if promoted {
            service.save(&list);
        }
        promoted
    };
    if promoted {
        publish(app);
        return;
    }

    let id = service.next_id();
    let image = match decoded {
        Some(image) => {
            let language = app.state::<SettingsStore>().language();
            let label = tr(language, if origin == RecentOrigin::Screenshot { Text::RecentsScreenshotName } else { Text::RecentsImageName });
            match service.store_image(&image, label, &id) {
                Ok(stored) => Some(stored),
                Err(error) => {
                    log::warn!("Recents: {error}");
                    return;
                }
            }
        }
        None => None,
    };
    let item = RecentItem {
        id,
        kind,
        origin,
        created_at_ms: now,
        text,
        protected_text: None,
        files,
        image,
        sensitive,
        source_app: change.source_app,
        fingerprint: print,
    };
    let view = service.add(item, limit);
    publish(app);
    if origin == RecentOrigin::Screenshot && settings.screenshot_peek {
        notch::hold_visible(app, SCREENSHOT_PEEK_HOLD);
        if let Err(error) = app.emit(RECENTS_SCREENSHOT_EVENT, &view) {
            log::warn!("Cannot announce the screenshot: {error}");
        }
    }
}

fn image_origin(screenshot_app: bool) -> RecentOrigin {
    if screenshot_app { RecentOrigin::Screenshot } else { RecentOrigin::Copy }
}

/// Files dropped on the notch (the shelf).
pub fn add_files(app: &AppHandle, paths: Vec<String>) {
    let dropped = paths.len();
    let paths: Vec<String> = paths.into_iter().filter(|path| Path::new(path).exists()).collect();
    log::debug!("Recents: {dropped} item(s) dropped on the notch, {} existing file(s) kept", paths.len());
    if paths.is_empty() {
        log::warn!("Recents: nothing to keep from the drop ({dropped} path(s), none found on disk)");
        return;
    }
    let service = app.state::<RecentsService>();
    let print = fingerprint(RecentKind::Files, paths.join("\n").as_bytes());
    let item = RecentItem {
        id: service.next_id(),
        kind: RecentKind::Files,
        origin: RecentOrigin::Shelf,
        created_at_ms: now_ms(),
        text: None,
        protected_text: None,
        files: paths,
        image: None,
        sensitive: false,
        source_app: None,
        fingerprint: print,
    };
    service.add(item, settings(app).limit as usize);
    publish(app);
}

/// Puts an item back on the clipboard and moves it to the top.
pub fn copy(app: &AppHandle, id: &str) -> Result<(), String> {
    let service = app.state::<RecentsService>();
    let language = app.state::<SettingsStore>().language();
    let item = service.item(id).ok_or_else(|| tr(language, Text::ErrorRecentGone).to_string())?;
    match item.kind {
        RecentKind::Text => platform::write_clipboard_text(item.text.as_deref().unwrap_or_default())?,
        RecentKind::Files => {
            let files = existing_files(&item);
            if files.is_empty() {
                return Err(tr(language, Text::ErrorRecentFilesMissing).to_string());
            }
            platform::copy_files_to_clipboard(&files)?;
            platform::mark_own_clipboard_write();
        }
        RecentKind::Image => {
            let path = image_path(&service, &item).ok_or_else(|| tr(language, Text::ErrorRecentGone).to_string())?;
            let png = fs::read(&path).map_err(|_| tr(language, Text::ErrorRecentGone).to_string())?;
            let decoded = image::decode_png(&png)?;
            platform::write_clipboard_image(&image::encode_dib(&decoded), &png)?;
        }
    }
    service.update(|list| {
        list.promote(id, now_ms());
        Vec::new()
    });
    publish(app);
    Ok(())
}

/// Opens the image or file with its default app.
pub fn open(app: &AppHandle, id: &str) -> Result<(), String> {
    let service = app.state::<RecentsService>();
    let language = app.state::<SettingsStore>().language();
    let item = service.item(id).ok_or_else(|| tr(language, Text::ErrorRecentGone).to_string())?;
    let target = match item.kind {
        RecentKind::Image => image_path(&service, &item).map(|path| path.to_string_lossy().into_owned()),
        RecentKind::Files => existing_files(&item).into_iter().next(),
        RecentKind::Text => None,
    }
    .ok_or_else(|| tr(language, Text::ErrorRecentFilesMissing).to_string())?;
    if item.files.len() > 1 {
        platform::reveal_in_file_manager(&target)?;
    } else {
        platform::shell_open(&target, ShellVerb::Open)?;
    }
    notch::collapse(app, false);
    Ok(())
}

/// Starts dragging an item into another app. Called while the mouse button
/// is held over the item; the drag runs on the UI thread.
pub fn drag(app: &AppHandle, id: &str) -> Result<(), String> {
    let service = app.state::<RecentsService>();
    let language = app.state::<SettingsStore>().language();
    let item = service.item(id).ok_or_else(|| tr(language, Text::ErrorRecentGone).to_string())?;
    let files = match item.kind {
        RecentKind::Image => image_path(&service, &item).map(|path| vec![path.to_string_lossy().into_owned()]).unwrap_or_default(),
        RecentKind::Files => existing_files(&item),
        RecentKind::Text => Vec::new(),
    };
    if item.kind != RecentKind::Text && files.is_empty() {
        return Err(tr(language, Text::ErrorRecentFilesMissing).to_string());
    }
    let text = item.text.unwrap_or_default();
    app.run_on_main_thread(move || {
        let outcome = if files.is_empty() { platform::start_drag_text(&text) } else { platform::start_drag_files(&files) };
        if let Err(error) = outcome {
            log::warn!("Recents: {error}");
        }
    })
    .map_err(|error| format!("Cannot start the drag: {error}"))
}

pub fn remove(app: &AppHandle, id: &str) {
    let service = app.state::<RecentsService>();
    service.update(|list| list.remove(id).into_iter().collect());
    publish(app);
}

pub fn clear(app: &AppHandle) {
    let service = app.state::<RecentsService>();
    service.update(|list| list.remove_where(|_| true));
    publish(app);
}

/// A smaller limit trims the list; turning password-manager items off
/// deletes those already kept.
pub fn sync_with_settings(app: &AppHandle, previous: &RecentsSettings) {
    let settings = settings(app);
    let service = app.state::<RecentsService>();
    let drop_sensitive = previous.keep_password_manager_items && !settings.keep_password_manager_items;
    if settings.limit >= previous.limit && !drop_sensitive {
        return;
    }
    service.update(|list| {
        let mut removed = if drop_sensitive { list.remove_where(|item| item.sensitive) } else { Vec::new() };
        removed.extend(list.trim(settings.limit as usize));
        removed
    });
    publish(app);
}

fn publish(app: &AppHandle) {
    let views = app.state::<RecentsService>().views();
    if let Err(error) = app.emit(RECENTS_CHANGED_EVENT, views) {
        log::warn!("Cannot send the Recents list: {error}");
    }
}

fn existing_files(item: &RecentItem) -> Vec<String> {
    item.files.iter().filter(|path| Path::new(path).exists()).cloned().collect()
}

fn image_path(service: &RecentsService, item: &RecentItem) -> Option<PathBuf> {
    let path = service.images_folder().join(&item.image.as_ref()?.file);
    path.exists().then_some(path)
}

/// "bitwarden.exe" → "Bitwarden".
fn readable_app_name(executable: &str) -> String {
    let stem = executable.strip_suffix(".exe").unwrap_or(executable);
    let mut characters = stem.chars();
    characters.next().map_or_else(String::new, |first| first.to_uppercase().chain(characters).collect())
}

fn conceal(text: &str) -> Result<String, String> {
    platform::protect_for_user(text.as_bytes()).map(|bytes| STANDARD.encode(bytes))
}

fn reveal(protected: &str) -> Result<String, String> {
    let bytes = STANDARD.decode(protected).map_err(|error| format!("Corrupt protected text: {error}"))?;
    let clear = platform::unprotect_for_user(&bytes)?;
    String::from_utf8(clear).map_err(|error| format!("Corrupt protected text: {error}"))
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_names_are_readable() {
        assert_eq!(readable_app_name("bitwarden.exe"), "Bitwarden");
        assert_eq!(readable_app_name("keepassxc"), "Keepassxc");
        assert_eq!(readable_app_name(""), "");
    }

    #[test]
    fn protected_texts_round_trip() {
        let concealed = conceal("hunter2").expect("DPAPI is available on Windows");
        assert!(!concealed.contains("hunter2"));
        assert_eq!(reveal(&concealed).unwrap(), "hunter2");
    }
}
