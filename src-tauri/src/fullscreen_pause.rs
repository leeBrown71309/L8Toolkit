//! Pause in fullscreen apps: while a game or a video fills a screen, the
//! search shortcut and Quick Accent let every key through to it, so a key
//! combination meant for the game never opens the notch.
//!
//! The notch's pointer watcher notices fullscreen apps (see `notch.rs`) and
//! calls `sync` when that changes; a settings change calls it too.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::hotkey;
use crate::notch;
use crate::settings::SettingsStore;

#[derive(Default)]
pub struct FullscreenPause {
    /// Read by the keyboard hook on every key, hence an atomic.
    active: AtomicBool,
    /// Serializes `sync` so the shortcut is never released and taken back out of order.
    sync_lock: Mutex<()>,
}

/// True while the shortcut and Quick Accent step aside for a fullscreen app.
pub fn is_active(app: &AppHandle) -> bool {
    app.state::<FullscreenPause>().active.load(Ordering::SeqCst)
}

/// Pauses or resumes according to the setting and the focused window.
pub fn sync(app: &AppHandle) {
    let pause = app.state::<FullscreenPause>();
    let _guard = pause.sync_lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let wanted = app.state::<SettingsStore>().read(|settings| settings.pause_in_fullscreen);
    let paused = wanted && notch::is_fullscreen_app_focused(app);
    if pause.active.swap(paused, Ordering::SeqCst) == paused {
        return;
    }
    if paused {
        log::info!("Fullscreen app focused: search shortcut and Quick Accent paused");
    } else {
        log::info!("Search shortcut and Quick Accent resumed");
    }
    hotkey::set_suspended(app, paused);
}
