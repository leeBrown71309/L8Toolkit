//! The settings window is created on demand and destroyed when closed, so it
//! costs no memory while the app idles in the tray.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::appearance;
use crate::settings::SettingsStore;

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
/// Tells an open settings window to show a section.
const NAVIGATE_EVENT: &str = "settings:navigate";

/// Sections other parts of the app can link to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Updates,
}

impl SettingsSection {
    fn key(self) -> &'static str {
        match self {
            SettingsSection::Updates => "updates",
        }
    }
}

pub fn open(app: &AppHandle) {
    show(app, None);
}

/// Opens the settings window on a given section.
pub fn open_section(app: &AppHandle, section: SettingsSection) {
    show(app, Some(section));
}

fn show(app: &AppHandle, section: Option<SettingsSection>) {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        if let Err(error) = window.set_focus() {
            log::warn!("Cannot focus the settings window: {error}");
        }
        if let Some(section) = section {
            if let Err(error) = window.emit(NAVIGATE_EVENT, section.key()) {
                log::warn!("Cannot show the {} settings: {error}", section.key());
            }
        }
        return;
    }
    let page = match section {
        Some(section) => format!("index.html?section={}", section.key()),
        None => "index.html".to_string(),
    };

    // Window creation must not block the event loop: do it from a worker.
    let theme = appearance::window_theme(app.state::<SettingsStore>().read(|settings| settings.appearance.theme));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let created = WebviewWindowBuilder::new(&app, SETTINGS_WINDOW_LABEL, WebviewUrl::App(page.into()))
            .title("L8 Toolkit")
            .inner_size(920.0, 660.0)
            .min_inner_size(780.0, 540.0)
            .theme(theme)
            .center()
            .focused(true)
            .build();
        if let Err(error) = created {
            log::error!("Cannot open the settings window: {error}");
        }
    });
}
