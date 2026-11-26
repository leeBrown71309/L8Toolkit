//! The settings window is created on demand and destroyed when closed, so it
//! costs no memory while the app idles in the tray.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::appearance;
use crate::settings::SettingsStore;

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
/// The page normally reports itself drawn well before this.
const SHOW_FALLBACK_DELAY: std::time::Duration = std::time::Duration::from_millis(1500);
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
        bring_to_front(app, &window);
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
    // The window starts hidden, on the theme's background: WebView2 would
    // otherwise show white until the page has loaded and styled itself. The
    // page calls `settings_window_ready` once drawn; a timer is the safety net.
    let theme_mode = app.state::<SettingsStore>().read(|settings| settings.appearance.theme);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let created = WebviewWindowBuilder::new(&app, SETTINGS_WINDOW_LABEL, WebviewUrl::App(page.into()))
            .title("L8 Toolkit")
            .inner_size(920.0, 660.0)
            .min_inner_size(780.0, 540.0)
            .theme(appearance::window_theme(theme_mode))
            .background_color(appearance::window_background(theme_mode))
            .visible(false)
            .center()
            .build();
        match created {
            Ok(_) => {
                tauri::async_runtime::spawn_blocking(move || {
                    std::thread::sleep(SHOW_FALLBACK_DELAY);
                    reveal(&app, "fallback timer");
                });
            }
            Err(error) => log::error!("Cannot open the settings window: {error}"),
        }
    });
}

/// Shows the settings window once its page is drawn (or after the fallback
/// delay); does nothing when it is already visible.
pub fn reveal(app: &AppHandle, reason: &str) {
    let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) else { return };
    if window.is_visible().unwrap_or(false) {
        return;
    }
    log::debug!("Settings window shown ({reason})");
    if let Err(error) = window.show() {
        log::warn!("Cannot show the settings window: {error}");
        return;
    }
    bring_to_front(app, &window);
}

/// Puts the window in front with the focus. A plain focus request is often
/// refused by Windows here: the window appears a moment after the click (once
/// its page is drawn), from a tray app that is no longer in the foreground.
fn bring_to_front(app: &AppHandle, window: &WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        let handle = hwnd.0 as isize;
        // Focus changes must run on the thread that owns the window.
        if let Err(error) = app.run_on_main_thread(move || crate::platform::force_foreground(handle)) {
            log::warn!("Cannot bring the settings window to the front: {error}");
        }
        return;
    }
    let _ = app;
    if let Err(error) = window.set_focus() {
        log::warn!("Cannot focus the settings window: {error}");
    }
}
