//! Runtime status surfaced in the settings window.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::hotkey::HotkeyState;
use crate::search::SearchService;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub hotkey_error: Option<String>,
    pub apps_indexed: usize,
    pub files_indexed: usize,
    pub indexing_files: bool,
    pub version: String,
    /// The development copy (`bun run app:dev`), not an installed release.
    pub dev_build: bool,
}

pub fn current(app: &AppHandle) -> AppStatus {
    let search = app.state::<SearchService>();
    AppStatus {
        hotkey_error: app.state::<HotkeyState>().last_error(),
        apps_indexed: search.app_count(),
        files_indexed: search.file_count(),
        indexing_files: search.is_indexing_files(),
        version: app.package_info().version.to_string(),
        dev_build: cfg!(debug_assertions),
    }
}
