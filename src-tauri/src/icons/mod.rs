//! Real app/file icons for search results, served as PNG data URLs.
//!
//! Shell icon extraction must happen on a COM STA thread and is not free
//! (a few ms each), so a single worker thread owns it and results are cached.

pub mod encoding;

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::platform;

/// Rendered at 2x the 28px display size so icons stay crisp on HiDPI screens.
const ICON_PIXEL_SIZE: i32 = 64;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// Extensions whose icon is specific to each file rather than to the type.
const PER_FILE_ICON_EXTENSIONS: &[&str] = &["exe", "lnk", "ico", "url", "appref-ms", "msc"];

struct IconRequest {
    target: String,
    cache_key: String,
    reply: Sender<Option<String>>,
}

#[derive(Clone)]
pub struct IconService {
    requests: Sender<IconRequest>,
    cache: Arc<Mutex<HashMap<String, Option<String>>>>,
}

impl IconService {
    pub fn start() -> Self {
        let (requests, receiver) = mpsc::channel::<IconRequest>();
        let cache: Arc<Mutex<HashMap<String, Option<String>>>> = Arc::default();
        let worker_cache = Arc::clone(&cache);

        let spawned = thread::Builder::new().name("icon-worker".into()).spawn(move || {
            let _apartment = platform::enter_com();
            for request in receiver {
                let icon = render_data_url(&request.target);
                worker_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(request.cache_key, icon.clone());
                // The requester may have timed out; nothing else to do then.
                let _ = request.reply.send(icon);
            }
        });
        if let Err(error) = spawned {
            log::error!("Cannot start the icon worker: {error}");
        }

        Self { requests, cache }
    }

    /// Blocking: call from a background thread.
    pub fn resolve(&self, target: &str) -> Option<String> {
        let cache_key = cache_key_for(target);
        if let Some(cached) = self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&cache_key) {
            return cached.clone();
        }
        let (reply, response) = mpsc::channel();
        let request = IconRequest { target: target.to_string(), cache_key, reply };
        if self.requests.send(request).is_err() {
            log::error!("Icon worker is not running");
            return None;
        }
        response.recv_timeout(REQUEST_TIMEOUT).unwrap_or_else(|_| {
            log::warn!("Icon request timed out for {target}");
            None
        })
    }
}

fn render_data_url(target: &str) -> Option<String> {
    let rendered = platform::extract_icon_rgba(target, ICON_PIXEL_SIZE)
        .and_then(|(pixels, width, height)| encoding::png_data_url(&pixels, width, height));
    match rendered {
        Ok(data_url) => Some(data_url),
        Err(error) => {
            log::debug!("No icon for {target}: {error}");
            None
        }
    }
}

/// Files of the same type share one cached icon; apps, folders and
/// executables each get their own.
fn cache_key_for(target: &str) -> String {
    if target.starts_with("shell:") {
        return target.to_string();
    }
    let path = Path::new(target);
    let extension = path.extension().map(|extension| extension.to_string_lossy().to_lowercase());
    match extension {
        Some(extension) if !PER_FILE_ICON_EXTENSIONS.contains(&extension.as_str()) && !path.is_dir() => {
            format!("extension:{extension}")
        }
        _ => target.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::cache_key_for;

    #[test]
    fn documents_share_icons_but_executables_do_not() {
        assert_eq!(cache_key_for("C:\\a\\report.PDF"), "extension:pdf");
        assert_eq!(cache_key_for("C:\\b\\other.pdf"), "extension:pdf");
        assert_eq!(cache_key_for("C:\\a\\tool.exe"), "c:\\a\\tool.exe");
        assert_eq!(cache_key_for("shell:AppsFolder\\App"), "shell:AppsFolder\\App");
    }
}
