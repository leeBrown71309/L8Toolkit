//! L8 Toolkit: a tray app hosting small widgets inside a virtual notch at the
//! top of the screen. The first widget is Spotlight (search apps, files, the
//! web, and quick calculations).

mod actions;
mod appearance;
mod audio;
mod autostart;
mod backdrop;
mod commands;
mod fullscreen_pause;
mod hotkey;
mod i18n;
mod icons;
mod indexer;
mod json_file;
mod media;
mod notch;
mod platform;
mod quick_accent;
mod recents;
mod search;
mod settings;
mod settings_sync;
mod settings_window;
mod status;
mod timer;
mod tray;
mod updates;
mod usage;

use std::error::Error;

use tauri::{AppHandle, Manager, RunEvent};

use crate::audio::AudioVisualizer;
use crate::backdrop::WallpaperCache;
use crate::fullscreen_pause::FullscreenPause;
use crate::hotkey::HotkeyState;
use crate::icons::IconService;
use crate::media::MediaService;
use crate::notch::NotchController;
use crate::quick_accent::QuickAccentService;
use crate::recents::RecentsService;
use crate::search::SearchService;
use crate::settings::SettingsStore;
use crate::timer::TimerService;
use crate::updates::UpdateService;
use crate::usage::UsageStore;

const SETTINGS_FILE_NAME: &str = "settings.json";
const USAGE_FILE_NAME: &str = "usage.json";
const RECENTS_FOLDER_NAME: &str = "recents";
const TOGGLE_SEARCH_ARGUMENT: &str = "--toggle-search";

pub fn run() {
    let app = tauri::Builder::default()
        // Launching the app a second time opens its settings instead of a
        // duplicate process fighting over the same hotkey. `--toggle-search`
        // lets other tools (mouse buttons, Stream Deck, scripts) open the notch.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|argument| argument == TOGGLE_SEARCH_ARGUMENT) {
                notch::toggle_from_hotkey(app);
            } else {
                settings_window::open(app);
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                // The test copy also logs this app's own details (Quick Accent decisions...).
                .level_for(
                    "l8_tools_kits_lib",
                    if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info },
                )
                .build(),
        )
        .plugin(hotkey::plugin())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::frequent_items,
            commands::execute_result,
            commands::resolve_icon,
            commands::notch_state,
            commands::notch_collapse,
            commands::get_wallpaper,
            commands::media_state,
            commands::media_command,
            commands::media_open_source,
            commands::audio_bars_set_visible,
            commands::update_state,
            commands::update_check,
            commands::update_install,
            commands::quick_accent_preview,
            commands::timer_state,
            commands::timer_command,
            commands::notch_set_layout,
            commands::notch_open_for_drop,
            commands::recents_list,
            commands::recents_copy,
            commands::recents_open,
            commands::recents_drag,
            commands::recents_remove,
            commands::recents_clear,
            commands::recents_add_files,
            commands::settings_window_ready,
            commands::get_settings,
            commands::update_settings,
            commands::rebuild_index,
            commands::open_settings,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("L8 Toolkit failed to start");

    app.run(|_app, event| {
        // Closing the settings window must not quit a tray app; only an
        // explicit exit (tray menu, "Quit" command) carries an exit code.
        if let RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let config_dir = app.path().app_config_dir()?;
    let data_dir = app.path().app_data_dir()?;

    app.manage(SettingsStore::load(config_dir.join(SETTINGS_FILE_NAME)));
    app.manage(UsageStore::load(data_dir.join(USAGE_FILE_NAME)));
    app.manage(SearchService::new());
    app.manage(IconService::start());
    app.manage(NotchController::default());
    app.manage(HotkeyState::default());
    app.manage(FullscreenPause::default());
    app.manage(WallpaperCache::new(app.path().app_cache_dir()?));
    app.manage(MediaService::default());
    app.manage(AudioVisualizer::default());
    app.manage(UpdateService::default());
    app.manage(QuickAccentService::default());
    app.manage(TimerService::default());
    app.manage(RecentsService::load(data_dir.join(RECENTS_FOLDER_NAME)));

    tray::create(app)?;
    notch::initialize(app)?;
    settings_sync::apply_on_startup(app);
    indexer::spawn_full_index(app);
    indexer::spawn_periodic_refresh(app);
    media::start_watcher(app);
    audio::start(app);
    updates::spawn_periodic_check(app);
    timer::start_watcher(app);
    recents::start(app);
    log::info!("L8 Toolkit {} ready", app.package_info().version);
    Ok(())
}
