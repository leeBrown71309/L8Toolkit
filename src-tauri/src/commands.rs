//! IPC commands exposed to the web layer. Each one is a thin adapter; the
//! logic lives in the dedicated modules.

use tauri::{AppHandle, Manager, State};

use crate::actions::{self, ActionOutcome, ExecuteAction};
use crate::audio::{self, AudioVisualizer};
use crate::quick_accent::{self, AccentLanguage, AccentPreviewRow};
use crate::icons::IconService;
use crate::i18n::{tr, Text};
use crate::indexer;
use crate::media::{MediaService, NowPlaying, PlaybackRequest};
use crate::platform::{self, ShellVerb};
use crate::backdrop::WallpaperCache;
use crate::notch::{self, NotchController, NotchSnapshot};
use crate::search::{SearchResult, SearchSection, SearchService};
use crate::settings::{AppSettings, SettingsStore};
use crate::settings_sync::{self, SettingsSnapshot};
use crate::settings_window;
use crate::timer::{self, TimerRequest, TimerService, TimerSnapshot};
use crate::updates::{self, UpdateService, UpdateState};
use crate::usage::UsageStore;

const FREQUENT_ITEMS_LIMIT: usize = 6;

#[tauri::command]
pub async fn search(
    query: String,
    search: State<'_, SearchService>,
    usage: State<'_, UsageStore>,
    settings: State<'_, SettingsStore>,
) -> Result<Vec<SearchSection>, String> {
    let engine = settings.read(|settings| settings.spotlight.web_search_engine);
    let language = settings.language();
    Ok(usage.with_lookup(|lookup| search.search(&query, lookup, engine, language)))
}

#[tauri::command]
pub async fn frequent_items(usage: State<'_, UsageStore>) -> Result<Vec<SearchResult>, String> {
    Ok(usage.frequent(FREQUENT_ITEMS_LIMIT))
}

#[tauri::command]
pub async fn execute_result(
    app: AppHandle,
    result: SearchResult,
    action: ExecuteAction,
) -> Result<ActionOutcome, String> {
    actions::execute(&app, &result, action)
}

#[tauri::command]
pub async fn resolve_icon(target: String, icons: State<'_, IconService>) -> Result<Option<String>, String> {
    let service = icons.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.resolve(&target))
        .await
        .map_err(|error| format!("Icon task failed: {error}"))
}

#[tauri::command]
pub fn notch_state(controller: State<'_, NotchController>) -> NotchSnapshot {
    controller.snapshot()
}

#[tauri::command]
pub fn notch_collapse(app: AppHandle, restore_focus: bool) {
    notch::collapse(&app, restore_focus);
}

/// Desktop wallpaper thumbnail for the Mica material.
#[tauri::command]
pub async fn get_wallpaper(cache: State<'_, WallpaperCache>) -> Result<Option<String>, String> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cache.data_url())
        .await
        .map_err(|error| format!("Wallpaper task failed: {error}"))?
}

#[tauri::command]
pub fn media_state(media: State<'_, MediaService>) -> Option<NowPlaying> {
    media.now_playing()
}

#[tauri::command]
pub async fn media_command(app: AppHandle, request: PlaybackRequest) -> Result<(), String> {
    let language = app.state::<SettingsStore>().language();
    tauri::async_runtime::spawn_blocking(move || app.state::<MediaService>().send(request, language))
        .await
        .map_err(|error| format!("Media task failed: {error}"))?
}

/// The notch shows (or stops showing) the audio bars. Ignored when the
/// settings turn them off.
#[tauri::command]
pub fn audio_bars_set_visible(app: AppHandle, visible: bool) {
    let wanted = visible && audio::bars_enabled(&app);
    app.state::<AudioVisualizer>().set_wanted(wanted);
}

/// Brings the player (Spotify, the browser...) to the front.
#[tauri::command]
pub async fn media_open_source(app: AppHandle) -> Result<(), String> {
    let target = app
        .state::<MediaService>()
        .now_playing()
        .and_then(|playing| playing.source_launch_target)
        .ok_or_else(|| tr(app.state::<SettingsStore>().language(), Text::ErrorPlayerUnknown).to_string())?;
    platform::shell_open(&target, ShellVerb::Open)?;
    notch::collapse(&app, false);
    Ok(())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsSnapshot {
    settings_sync::snapshot(&app)
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, settings: AppSettings) -> Result<SettingsSnapshot, String> {
    settings_sync::apply(&app, settings)
}

#[tauri::command]
pub async fn rebuild_index(app: AppHandle) -> Result<(), String> {
    indexer::spawn_full_index(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    notch::collapse(&app, false);
    settings_window::open(&app);
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn update_state(service: State<'_, UpdateService>) -> UpdateState {
    service.state()
}

/// Asks GitHub for a newer version (Settings › Updates › Check now).
#[tauri::command]
pub async fn update_check(app: AppHandle) -> UpdateState {
    updates::check(&app).await
}

/// Downloads, verifies and installs the newest version; the app restarts.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    updates::install(&app).await
}

/// The settings page has drawn its first content: the window can appear.
#[tauri::command]
pub fn settings_window_ready(app: AppHandle) {
    settings_window::reveal(&app, "page ready");
}

/// Letters and their accents for these languages (Settings › Quick Accent).
#[tauri::command]
pub fn quick_accent_preview(languages: Vec<AccentLanguage>) -> Vec<AccentPreviewRow> {
    quick_accent::preview(&languages)
}

#[tauri::command]
pub fn timer_state(timer: State<'_, TimerService>) -> TimerSnapshot {
    timer.snapshot()
}

/// Start, pause, resume, reset, add time or skip (Timer tab).
#[tauri::command]
pub fn timer_command(app: AppHandle, request: TimerRequest) -> TimerSnapshot {
    timer::apply(&app, request)
}

/// The folded notch shows a second activity in a bubble beside it: the
/// pointer resting on that bubble opens the panel on it.
#[tauri::command]
pub fn notch_set_layout(controller: State<'_, NotchController>, bubble: bool) {
    controller.set_bubble(bubble);
}
