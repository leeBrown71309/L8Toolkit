//! In-app updates: signed installers published on GitHub Releases.
//!
//! The endpoint and the public key live in `tauri.conf.json`
//! (`plugins.updater`). The plugin refuses an installer whose signature does
//! not match that key, and any version that is not strictly newer, so a
//! tampered manifest can neither install foreign code nor downgrade the app.
//!
//! Installed copies check shortly after startup and then every few hours;
//! when a new version exists the user is told once (Windows notification,
//! tray item) and installs it from Settings > Updates in one click.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::i18n::{tr, tr_with, Text};
use crate::settings::SettingsStore;
use crate::tray;

pub const UPDATE_STATE_EVENT: &str = "update:state";
/// Leaves startup (indexing, media) alone before touching the network.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
/// Progress events are throttled to whole percents.
const PROGRESS_STEPS: u64 = 100;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "status")]
pub enum UpdateState {
    /// Not checked yet in this session.
    Idle,
    Checking,
    UpToDate,
    Available { version: String, notes: String, published_at_ms: Option<i64> },
    Downloading { version: String, downloaded_bytes: u64, total_bytes: Option<u64> },
    /// The installer is starting; the app closes and restarts on its own.
    Installing { version: String },
    Failed { message: String },
}

pub struct UpdateService {
    state: Mutex<UpdateState>,
    /// One check or install at a time (a timer check during a click...).
    busy: AtomicBool,
    /// Version the user was already told about, so it is announced once.
    announced: Mutex<Option<String>>,
}

impl Default for UpdateService {
    fn default() -> Self {
        Self { state: Mutex::new(UpdateState::Idle), busy: AtomicBool::new(false), announced: Mutex::new(None) }
    }
}

impl UpdateService {
    pub fn state(&self) -> UpdateState {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    pub fn available_version(&self) -> Option<String> {
        match self.state() {
            UpdateState::Available { version, .. } => Some(version),
            _ => None,
        }
    }

    fn try_begin(&self) -> bool {
        !self.busy.swap(true, Ordering::SeqCst)
    }

    fn end(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }
}

/// Releases the busy flag whatever way the task ends.
struct BusyGuard<'a>(&'a UpdateService);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.end();
    }
}

fn set_state(app: &AppHandle, next: UpdateState) {
    let service = app.state::<UpdateService>();
    *service.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = next.clone();
    if let Err(error) = app.emit(UPDATE_STATE_EVENT, &next) {
        log::warn!("Cannot broadcast the update state: {error}");
    }
}

/// Periodic background checks. Development builds skip them: their version
/// is not a release and they would only log network noise.
pub fn spawn_periodic_check(app: &AppHandle) {
    if cfg!(debug_assertions) {
        log::info!("Automatic update checks are off in development builds");
        return;
    }
    let app = app.clone();
    // A plain thread that sleeps between checks keeps no async worker busy.
    let spawned = std::thread::Builder::new().name("update-checker".into()).spawn(move || {
        std::thread::sleep(FIRST_CHECK_DELAY);
        loop {
            tauri::async_runtime::block_on(check(&app));
            std::thread::sleep(CHECK_INTERVAL);
        }
    });
    if let Err(error) = spawned {
        log::error!("Cannot start the update checks: {error}");
    }
}

/// Asks GitHub whether a newer version exists and publishes the result.
pub async fn check(app: &AppHandle) -> UpdateState {
    let service = app.state::<UpdateService>();
    if !service.try_begin() {
        return service.state();
    }
    let _busy = BusyGuard(&service);
    set_state(app, UpdateState::Checking);
    let next = match fetch_update(app).await {
        Ok(Some(update)) => available_state(&update),
        Ok(None) => UpdateState::UpToDate,
        Err(error) => {
            log::warn!("Update check failed: {error}");
            UpdateState::Failed { message: tr(app.state::<SettingsStore>().language(), Text::UpdateCheckFailed).into() }
        }
    };
    set_state(app, next.clone());
    if let UpdateState::Available { version, .. } = &next {
        announce(app, version);
    }
    next
}

/// Downloads and starts the installer of the newest version. On Windows the
/// plugin closes the app once the installer runs; the installer restarts it.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let language = app.state::<SettingsStore>().language();
    let service = app.state::<UpdateService>();
    if !service.try_begin() {
        return Err(tr(language, Text::UpdateBusy).into());
    }
    let _busy = BusyGuard(&service);

    let update = match fetch_update(app).await {
        Ok(Some(update)) => update,
        Ok(None) => {
            set_state(app, UpdateState::UpToDate);
            return Ok(());
        }
        Err(error) => {
            log::warn!("Update check before install failed: {error}");
            let message = tr(language, Text::UpdateCheckFailed).to_string();
            set_state(app, UpdateState::Failed { message: message.clone() });
            return Err(message);
        }
    };

    let version = update.version.clone();
    log::info!("Downloading L8 Toolkit {version}");
    set_state(app, UpdateState::Downloading { version: version.clone(), downloaded_bytes: 0, total_bytes: None });
    let mut downloaded: u64 = 0;
    let mut last_step: Option<u64> = None;
    let progress_app = app.clone();
    let progress_version = version.clone();
    let finished_app = app.clone();
    let finished_version = version.clone();
    let outcome = update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let step = total.filter(|total| *total > 0).map(|total| downloaded * PROGRESS_STEPS / total);
                if step.is_none() || step != last_step {
                    last_step = step;
                    set_state(
                        &progress_app,
                        UpdateState::Downloading {
                            version: progress_version.clone(),
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                        },
                    );
                }
            },
            move || {
                log::info!("Installing L8 Toolkit {finished_version}");
                set_state(&finished_app, UpdateState::Installing { version: finished_version });
            },
        )
        .await;

    match outcome {
        // Only reached where the platform does not exit for the installer.
        Ok(()) => {
            app.restart();
        }
        Err(error) => {
            log::warn!("Installing update {version} failed: {error}");
            let message = tr(language, Text::UpdateInstallFailed).to_string();
            set_state(app, UpdateState::Failed { message: message.clone() });
            Err(message)
        }
    }
}

async fn fetch_update(app: &AppHandle) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    updater.check().await.map_err(|error| error.to_string())
}

fn available_state(update: &Update) -> UpdateState {
    UpdateState::Available {
        version: update.version.clone(),
        notes: update.body.clone().unwrap_or_default(),
        published_at_ms: update.date.map(|date| date.unix_timestamp() * 1000),
    }
}

/// Tells the user once per version: a Windows notification and a tray item.
fn announce(app: &AppHandle, version: &str) {
    let service = app.state::<UpdateService>();
    {
        let mut announced = service.announced.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if announced.as_deref() == Some(version) {
            return;
        }
        *announced = Some(version.to_string());
    }
    log::info!("L8 Toolkit {version} is available");
    tray::refresh_menu(app);
    let language = app.state::<SettingsStore>().language();
    let shown = app
        .notification()
        .builder()
        .title(tr_with(language, Text::UpdateNotificationTitle, &[("version", version)]))
        .body(tr(language, Text::UpdateNotificationBody))
        .show();
    if let Err(error) = shown {
        log::warn!("Cannot show the update notification: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateState;

    #[test]
    fn states_match_the_web_payloads() {
        let available = UpdateState::Available { version: "0.2.0".into(), notes: "Faster".into(), published_at_ms: Some(1) };
        assert_eq!(
            serde_json::to_value(&available).unwrap(),
            serde_json::json!({ "status": "available", "version": "0.2.0", "notes": "Faster", "publishedAtMs": 1 })
        );
        let downloading = UpdateState::Downloading { version: "0.2.0".into(), downloaded_bytes: 5, total_bytes: None };
        assert_eq!(
            serde_json::to_value(&downloading).unwrap(),
            serde_json::json!({ "status": "downloading", "version": "0.2.0", "downloadedBytes": 5, "totalBytes": null })
        );
        assert_eq!(serde_json::to_value(UpdateState::UpToDate).unwrap(), serde_json::json!({ "status": "upToDate" }));
    }
}
