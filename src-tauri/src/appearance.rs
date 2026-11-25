//! Applies the appearance settings to native windows: the window theme drives
//! the title bar and `prefers-color-scheme`. Colors and materials are drawn
//! by the web layer (see `backdrop.rs` for why materials are not native).

use tauri::{AppHandle, Manager, Theme};

use crate::notch::NOTCH_WINDOW_LABEL;
use crate::settings::{SettingsStore, ThemeMode};
use crate::settings_window::SETTINGS_WINDOW_LABEL;

/// `None` lets the window follow Windows' own light/dark setting.
pub fn window_theme(theme: ThemeMode) -> Option<Theme> {
    match theme {
        ThemeMode::System => None,
        ThemeMode::Dark => Some(Theme::Dark),
        ThemeMode::Light => Some(Theme::Light),
    }
}

pub fn apply(app: &AppHandle) {
    let theme = window_theme(app.state::<SettingsStore>().read(|settings| settings.appearance.theme));
    for label in [NOTCH_WINDOW_LABEL, SETTINGS_WINDOW_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            if let Err(error) = window.set_theme(theme) {
                log::warn!("Cannot set the {label} window theme: {error}");
            }
        }
    }
}
