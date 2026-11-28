//! Background (re)indexing of apps and files.
//!
//! Indexes refresh when something changes rather than on a tight timer:
//! the Start menu folders (where installers put their shortcuts) and the
//! indexed file folders are watched by Windows, at no CPU cost while idle.
//! A burst of changes (an installer, a download) leads to one pass once it
//! settles. Store apps add no shortcut there, so opening the notch also
//! refreshes an app index that has grown old. A slow periodic pass stays as
//! a safety net.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::platform;
use crate::search::apps::build_app_entries;
use crate::search::files::{collect_roots, index_roots, MAX_INDEXED_ENTRIES};
use crate::search::SearchService;
use crate::settings::SettingsStore;
use crate::status;

pub const INDEX_STATUS_EVENT: &str = "index:status";
/// Safety net: whatever the watchers missed shows up at the latest after this.
const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// An installer writes its shortcuts in a burst: wait for it to finish.
const APP_CHANGES_SETTLE: Duration = Duration::from_secs(3);
const APP_REINDEX_MIN_GAP: Duration = Duration::from_secs(10);
/// Downloads and saves touch the file folders often: batch them.
const FILE_CHANGES_SETTLE: Duration = Duration::from_secs(10);
const FILE_REINDEX_MIN_GAP: Duration = Duration::from_secs(120);
/// Opening the notch refreshes an app index older than this (Store apps).
const APP_INDEX_MAX_AGE_ON_OPEN: Duration = Duration::from_secs(120);

/// When the app index was last rebuilt (Unix ms), for the refresh on open.
static APPS_INDEXED_AT_MS: AtomicU64 = AtomicU64::new(0);

pub fn spawn_full_index(app: &AppHandle) {
    spawn_app_index(app);
    spawn_file_index(app);
}

pub fn spawn_app_index(app: &AppHandle) {
    let app = app.clone();
    spawn_named("app-indexer", move || {
        APPS_INDEXED_AT_MS.store(unix_ms(), Ordering::SeqCst);
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

/// Rebuilds the app index in the background when it is older than a couple
/// of minutes: Store apps installed meanwhile become searchable. Cheap to call
/// every time the notch opens.
pub fn refresh_apps_if_stale(app: &AppHandle) {
    let indexed_at = APPS_INDEXED_AT_MS.load(Ordering::SeqCst);
    let age = Duration::from_millis(unix_ms().saturating_sub(indexed_at));
    if indexed_at != 0 && age >= APP_INDEX_MAX_AGE_ON_OPEN {
        spawn_app_index(app);
    }
}

/// Watches the Start menu folders (new apps) and the indexed file folders
/// (new files), and re-indexes what changed once the changes settle.
pub fn spawn_change_watchers(app: &AppHandle) {
    let start_menus = start_menu_folders();
    let extra_folders = app.state::<SettingsStore>().read(|settings| settings.spotlight.extra_folders.clone());
    let file_roots = collect_roots(&extra_folders);
    let app_count = start_menus.len();

    let apps_handle = app.clone();
    let apps = Debouncer::spawn("app-reindex", APP_CHANGES_SETTLE, APP_REINDEX_MIN_GAP, move || {
        log::debug!("Start menu changed: re-indexing apps");
        spawn_app_index(&apps_handle);
    });
    let files_handle = app.clone();
    let files = Debouncer::spawn("file-reindex", FILE_CHANGES_SETTLE, FILE_REINDEX_MIN_GAP, move || {
        log::debug!("Indexed folders changed: re-indexing files");
        spawn_file_index(&files_handle);
    });
    let (Some(apps), Some(files)) = (apps, files) else {
        return;
    };

    let folders: Vec<PathBuf> = start_menus.into_iter().chain(file_roots).collect();
    let watched = platform::watch_folders(
        folders,
        Box::new(move |index| if index < app_count { apps.poke() } else { files.poke() }),
    );
    if let Err(error) = watched {
        log::warn!("New apps and files will only appear at the periodic refresh: {error}");
    }
}

/// Where installers put the shortcuts that make an app appear in Start.
fn start_menu_folders() -> Vec<PathBuf> {
    let programs = PathBuf::from("Microsoft").join("Windows").join("Start Menu").join("Programs");
    [dirs::data_dir(), std::env::var_os("ProgramData").map(PathBuf::from)]
        .into_iter()
        .flatten()
        .map(|base| base.join(&programs))
        .filter(|folder| folder.is_dir())
        .collect()
}

/// Runs a task once a burst of pokes has settled, and never more often than
/// `min_gap`, from a thread of its own.
struct Debouncer {
    pokes: Sender<()>,
}

impl Debouncer {
    fn spawn(name: &str, settle: Duration, min_gap: Duration, task: impl Fn() + Send + 'static) -> Option<Self> {
        let (pokes, received) = mpsc::channel::<()>();
        let spawned = thread::Builder::new().name(name.to_string()).spawn(move || {
            let mut last_run: Option<Instant> = None;
            // Wait for a first poke, then for the burst to settle.
            while received.recv().is_ok() {
                loop {
                    match received.recv_timeout(settle) {
                        Ok(()) => continue,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                thread::sleep(wait_before_run(last_run.map(|run| run.elapsed()), min_gap));
                // Pokes that came during the wait are covered by this run.
                while received.try_recv().is_ok() {}
                task();
                last_run = Some(Instant::now());
            }
        });
        match spawned {
            Ok(_) => Some(Debouncer { pokes }),
            Err(error) => {
                log::error!("Cannot start {name}: {error}");
                None
            }
        }
    }

    fn poke(&self) {
        // The thread only stops with the app.
        let _ = self.pokes.send(());
    }
}

/// How long to wait so two runs are at least `min_gap` apart.
fn wait_before_run(since_last_run: Option<Duration>, min_gap: Duration) -> Duration {
    since_last_run.map_or(Duration::ZERO, |elapsed| min_gap.saturating_sub(elapsed))
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_millis() as u64)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_are_kept_apart_by_the_minimum_gap() {
        let gap = Duration::from_secs(120);
        assert_eq!(wait_before_run(None, gap), Duration::ZERO, "the first run is immediate");
        assert_eq!(wait_before_run(Some(Duration::from_secs(30)), gap), Duration::from_secs(90));
        assert_eq!(wait_before_run(Some(Duration::from_secs(300)), gap), Duration::ZERO);
    }

    #[test]
    fn start_menu_folders_exist() {
        assert!(start_menu_folders().iter().all(|folder| folder.ends_with("Programs")));
    }
}
