//! Applies the appearance settings to native windows: the window theme drives
//! the title bar and `prefers-color-scheme`. Colors and materials are drawn
//! by the web layer (see `backdrop.rs` for why materials are not native).

use tauri::window::Color;
use tauri::{AppHandle, Manager, Theme};

use crate::notch::NOTCH_WINDOW_LABEL;
use crate::platform;
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

/// Background of an opaque window before its page paints: the same ink as
/// `--ink-1` in tokens.css, so the first frames are never white.
pub fn window_background(theme: ThemeMode) -> Color {
    let light = match theme {
        ThemeMode::Light => true,
        ThemeMode::Dark => false,
        ThemeMode::System => platform::system_prefers_light(),
    };
    if light {
        Color(0xf6, 0xf6, 0xf4, 0xff)
    } else {
        Color(0x0b, 0x0b, 0x0d, 0xff)
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
