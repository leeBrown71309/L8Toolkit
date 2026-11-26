//! Applies a settings change to the running app (shortcut, startup, notch,
//! index) and broadcasts the new state to every window.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, AudioVisualizer};
use crate::quick_accent::QuickAccentService;
use crate::appearance;
use crate::fullscreen_pause;
use crate::hotkey;
use crate::i18n::Language;
use crate::tray;
use crate::indexer;
use crate::notch;
use crate::platform;
use crate::settings::{AppSettings, SettingsStore};
use crate::status::{self, AppStatus};

pub const SETTINGS_CHANGED_EVENT: &str = "settings:changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub settings: AppSettings,
    pub status: AppStatus,
    /// Windows accent color, used when the accent is set to "system".
    pub system_accent_color: Option<String>,
    /// Language actually used ("system" resolved against Windows).
    pub effective_language: Language,
}

pub fn snapshot(app: &AppHandle) -> SettingsSnapshot {
    let store = app.state::<SettingsStore>();
    SettingsSnapshot {
        settings: store.get(),
        status: status::current(app),
        system_accent_color: platform::system_accent_color(),
        effective_language: store.language(),
    }
}

/// Applies the startup state once, right after launch.
pub fn apply_on_startup(app: &AppHandle) {
    let settings = app.state::<SettingsStore>().get();
    if settings.spotlight.enabled {
        // A failure is recorded in the hotkey state and shown in Settings.
        let _ = hotkey::register_spotlight(app, &settings.spotlight.hotkey);
    }
    if let Err(error) = crate::autostart::sync(app, settings.launch_at_startup) {
        log::warn!("{error}");
    }
    appearance::apply(app);
    app.state::<QuickAccentService>().configure(app, &settings.quick_accent);
}

pub fn apply(app: &AppHandle, requested: AppSettings) -> Result<SettingsSnapshot, String> {
    let store = app.state::<SettingsStore>();
    let previous = store.get();
    let mut next = requested;
    next.spotlight.hotkey = next.spotlight.hotkey.trim().to_string();

    apply_spotlight_shortcut(app, &previous, &mut next);

    if next.launch_at_startup != previous.launch_at_startup {
        if let Err(error) = crate::autostart::sync(app, next.launch_at_startup) {
            next.launch_at_startup = previous.launch_at_startup;
            return Err(error);
        }
    }

    store.replace(next.clone())?;

    if next.appearance != previous.appearance {
        appearance::apply(app);
    }
    if next.language != previous.language {
        tray::refresh_menu(app);
    }
    if !next.spotlight.enabled {
        notch::collapse(app, true);
    }
    if next.pause_in_fullscreen != previous.pause_in_fullscreen {
        fullscreen_pause::sync(app);
    }
    if next.quick_accent != previous.quick_accent {
        app.state::<QuickAccentService>().configure(app, &next.quick_accent);
    }
    if !audio::bars_enabled(app) {
        app.state::<AudioVisualizer>().set_wanted(false);
    }
    notch::sync_idle_visibility(app);

    let index_changed = next.spotlight.index_files != previous.spotlight.index_files
        || next.spotlight.extra_folders != previous.spotlight.extra_folders;
    if index_changed {
        indexer::spawn_file_index(app);
    }

    let snapshot = snapshot(app);
    if let Err(error) = app.emit(SETTINGS_CHANGED_EVENT, &snapshot) {
        log::warn!("Cannot broadcast settings: {error}");
    }
    Ok(snapshot)
}

/// Keeps the previous shortcut when the new one cannot be registered; the
/// error stays visible through `AppStatus::hotkey_error`.
fn apply_spotlight_shortcut(app: &AppHandle, previous: &AppSettings, next: &mut AppSettings) {
    if !next.spotlight.enabled {
        hotkey::unregister_spotlight(app);
        return;
    }
    let shortcut_changed = next.spotlight.hotkey != previous.spotlight.hotkey;
    let re_enabled = !previous.spotlight.enabled;
    if !(shortcut_changed || re_enabled) {
        return;
    }
    if hotkey::register_spotlight(app, &next.spotlight.hotkey).is_err() && shortcut_changed {
        next.spotlight.hotkey = previous.spotlight.hotkey.clone();
    }
}
