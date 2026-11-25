//! System tray entry: the app has no taskbar button, so this is its home.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::i18n::{tr, tr_with, Text};
use crate::notch::{self, OpenSource};
use crate::settings::SettingsStore;
use crate::settings_window::{self, SettingsSection};
use crate::updates::UpdateService;

const TRAY_ID: &str = "l8-tray";
const MENU_OPEN_SEARCH: &str = "open-search";
const MENU_SETTINGS: &str = "settings";
const MENU_INSTALL_UPDATE: &str = "install-update";
const MENU_QUIT: &str = "quit";

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let language = app.state::<SettingsStore>().language();
    let open_search = MenuItem::with_id(app, MENU_OPEN_SEARCH, tr(language, Text::TrayOpenSearch), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, tr(language, Text::TraySettings), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, tr(language, Text::TrayQuit), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_search, &settings, &separator, &quit])?;
    // A pending update gets its own entry, right above Quit.
    if let Some(version) = app.state::<UpdateService>().available_version() {
        let label = tr_with(language, Text::TrayInstallUpdate, &[("version", &version)]);
        let install = MenuItem::with_id(app, MENU_INSTALL_UPDATE, label, true, None::<&str>)?;
        menu.insert(&install, 2)?;
    }
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    // "L8 Toolkit Dev" for the development copy, so the two never get mixed up.
    let product_name = app.config().product_name.clone().unwrap_or_else(|| "L8 Toolkit".to_string());
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(product_name)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_SEARCH => notch::open(app, OpenSource::Tray),
            MENU_SETTINGS => settings_window::open(app),
            MENU_INSTALL_UPDATE => settings_window::open_section(app, SettingsSection::Updates),
            MENU_QUIT => app.exit(0),
            other => log::warn!("Unknown tray menu item: {other}"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                notch::open(tray.app_handle(), OpenSource::Tray);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu after a language change or when an update shows up.
pub fn refresh_menu(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        log::warn!("The tray icon does not exist");
        return;
    };
    let outcome = build_menu(app).and_then(|menu| tray.set_menu(Some(menu)));
    if let Err(error) = outcome {
        log::warn!("Cannot translate the tray menu: {error}");
    }
}
