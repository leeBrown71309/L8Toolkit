//! Background (re)indexing of apps and files.

use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::platform;
use crate::search::apps::build_app_entries;
use crate::search::files::{collect_roots, index_roots, MAX_INDEXED_ENTRIES};
use crate::search::SearchService;
use crate::settings::SettingsStore;
use crate::status;

pub const INDEX_STATUS_EVENT: &str = "index:status";
/// New installs and new files show up without restarting the app.
const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub fn spawn_full_index(app: &AppHandle) {
    spawn_app_index(app);
    spawn_file_index(app);
}

pub fn spawn_app_index(app: &AppHandle) {
    let app = app.clone();
    spawn_named("app-indexer", move || {
        let started = Instant::now();
        match platform::enumerate_installed_apps() {
            Ok(installed) => {
                let entries = build_app_entries(installed);
                log::info!("Indexed {} apps in {:?}", entries.len(), started.elapsed());
                app.state::<SearchService>().replace_apps(entries);
            }
            Err(error) => log::error!("App indexing failed: {error}"),
        }
        emit_status(&app);
    });
}

pub fn spawn_file_index(app: &AppHandle) {
    if !app.state::<SearchService>().try_begin_file_indexing() {
        // A pass is running; it will run once more with the latest settings.
        return;
    }
    emit_status(app);
    let app = app.clone();
    spawn_named("file-indexer", move || loop {
        let (enabled, extra_folders) = app
            .state::<SettingsStore>()
            .read(|settings| (settings.spotlight.index_files, settings.spotlight.extra_folders.clone()));
        let started = Instant::now();
        let entries = if enabled {
            index_roots(&collect_roots(&extra_folders), MAX_INDEXED_ENTRIES)
        } else {
            Vec::new()
        };
        log::info!("Indexed {} files and folders in {:?}", entries.len(), started.elapsed());

        let search = app.state::<SearchService>();
        search.replace_files(entries);
        if !search.finish_file_indexing() {
            emit_status(&app);
            break;
        }
    });
}

pub fn spawn_periodic_refresh(app: &AppHandle) {
    let app = app.clone();
    spawn_named("index-refresh", move || loop {
        thread::sleep(REFRESH_INTERVAL);
        spawn_full_index(&app);
    });
}

fn emit_status(app: &AppHandle) {
    if let Err(error) = app.emit(INDEX_STATUS_EVENT, status::current(app)) {
        log::warn!("Cannot broadcast index status: {error}");
    }
}

fn spawn_named(name: &str, task: impl FnOnce() + Send + 'static) {
    if let Err(error) = thread::Builder::new().name(name.to_string()).spawn(task) {
        log::error!("Cannot start {name}: {error}");
    }
}
