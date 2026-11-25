//! Executes what the user picked in the Spotlight list or its action menu.

use std::path::Path;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::i18n::{tr, tr_with, Language, Text};
use crate::indexer;
use crate::notch;
use crate::platform::{self, ShellVerb};
use crate::search::system_actions as commands;
use crate::search::{ResultKind, SearchResult, SearchService};
use crate::settings::SettingsStore;
use crate::settings_window;
use crate::usage::{is_pinnable, UsageStore};

/// Time for the previous window to regain focus before Ctrl+V is sent.
const PASTE_FOCUS_DELAY: Duration = Duration::from_millis(160);
const MANAGE_APPS_URI: &str = "ms-settings:appsfeatures";

/// Clipboard-only actions (copy path, copy URL) run in the web layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecuteAction {
    Open,
    RunAsAdmin,
    Reveal,
    OpenWith,
    OpenInTerminal,
    Properties,
    CopyFile,
    MoveToRecycleBin,
    Pin,
    Unpin,
    Forget,
    ManageApps,
    /// Closes the notch and pastes the clipboard into the previous window.
    PasteClipboard,
}

/// Tells the UI whether the notch closed or what to confirm in place.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "type")]
pub enum ActionOutcome {
    Dismissed,
    Done { message: String, refresh: bool },
}

fn done(message: &str, refresh: bool) -> ActionOutcome {
    ActionOutcome::Done { message: message.to_string(), refresh }
}

pub fn execute(app: &AppHandle, result: &SearchResult, action: ExecuteAction) -> Result<ActionOutcome, String> {
    let language = app.state::<SettingsStore>().language();
    match action {
        ExecuteAction::Open => open(app, result, language),
        ExecuteAction::RunAsAdmin => {
            ensure(result.kind == ResultKind::App || is_executable(&result.target), tr(language, Text::ErrorOnlyProgramsElevate))?;
            platform::shell_open(&result.target, ShellVerb::RunAsAdmin)?;
            launched(app, result)
        }
        ExecuteAction::Reveal => {
            platform::reveal_in_file_manager(&location_of(result, language)?)?;
            dismiss(app)
        }
        ExecuteAction::OpenWith => {
            ensure(result.kind == ResultKind::File, tr(language, Text::ErrorOpenWithFilesOnly))?;
            platform::open_with_dialog(&result.target)?;
            dismiss(app)
        }
        ExecuteAction::OpenInTerminal => {
            platform::open_in_terminal(&working_directory_of(result, language)?)?;
            dismiss(app)
        }
        ExecuteAction::Properties => {
            platform::show_properties(&location_of(result, language)?)?;
            dismiss(app)
        }
        ExecuteAction::CopyFile => {
            platform::copy_files_to_clipboard(&[location_of(result, language)?])?;
            Ok(done(tr(language, Text::OutcomeFileCopied), false))
        }
        ExecuteAction::MoveToRecycleBin => {
            ensure(matches!(result.kind, ResultKind::File | ResultKind::Folder), tr(language, Text::ErrorOnlyFilesDeletable))?;
            platform::move_to_recycle_bin(&result.target)?;
            app.state::<SearchService>().forget_path(&result.target);
            app.state::<UsageStore>().forget(&result.target);
            Ok(done(tr(language, Text::OutcomeMovedToRecycleBin), true))
        }
        ExecuteAction::Pin | ExecuteAction::Unpin => {
            ensure(is_pinnable(result.kind), tr(language, Text::ErrorOnlyPinnable))?;
            let pin = action == ExecuteAction::Pin;
            app.state::<UsageStore>().set_pinned(result, pin);
            Ok(done(tr(language, if pin { Text::OutcomePinned } else { Text::OutcomeUnpinned }), true))
        }
        ExecuteAction::Forget => {
            app.state::<UsageStore>().forget(&result.target);
            Ok(done(tr(language, Text::OutcomeRemovedFromFrequent), true))
        }
        ExecuteAction::ManageApps => {
            platform::shell_open(MANAGE_APPS_URI, ShellVerb::Open)?;
            dismiss(app)
        }
        ExecuteAction::PasteClipboard => paste_into_previous_window(app),
    }
}

fn open(app: &AppHandle, result: &SearchResult, language: Language) -> Result<ActionOutcome, String> {
    match result.kind {
        ResultKind::App | ResultKind::File | ResultKind::Folder => {
            platform::shell_open(&result.target, ShellVerb::Open)?;
            launched(app, result)
        }
        ResultKind::Url | ResultKind::Web => {
            platform::shell_open(&result.target, ShellVerb::Open)?;
            dismiss(app)
        }
        ResultKind::Command => {
            let outcome = run_command(app, &result.target, language)?;
            if commands::is_trackable(&result.target) {
                app.state::<UsageStore>().record(result);
            }
            Ok(outcome)
        }
        // Copying the value is handled by the web layer (clipboard access).
        ResultKind::Calculator => Ok(done(tr(language, Text::OutcomeCopied), false)),
    }
}

/// Remembers the launch and lets the launched app keep keyboard focus.
fn launched(app: &AppHandle, result: &SearchResult) -> Result<ActionOutcome, String> {
    app.state::<UsageStore>().record(result);
    dismiss(app)
}

fn dismiss(app: &AppHandle) -> Result<ActionOutcome, String> {
    notch::collapse(app, false);
    Ok(ActionOutcome::Dismissed)
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_string()) }
}

/// The file behind a result (the executable for desktop apps).
fn location_of(result: &SearchResult, language: Language) -> Result<String, String> {
    result
        .location
        .clone()
        .ok_or_else(|| tr_with(language, Text::ErrorNoFileOnDisk, &[("title", &result.title)]))
}

fn working_directory_of(result: &SearchResult, language: Language) -> Result<String, String> {
    let location = location_of(result, language)?;
    if result.kind == ResultKind::Folder {
        return Ok(location);
    }
    Path::new(&location)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .ok_or_else(|| format!("'{location}' has no parent folder"))
}

fn is_executable(path: &str) -> bool {
    let lowercase = path.to_lowercase();
    [".exe", ".bat", ".cmd", ".msi", ".lnk"].iter().any(|extension| lowercase.ends_with(extension))
}

/// The web layer already wrote the clipboard; give focus back and paste.
fn paste_into_previous_window(app: &AppHandle) -> Result<ActionOutcome, String> {
    notch::collapse(app, true);
    thread::spawn(|| {
        thread::sleep(PASTE_FOCUS_DELAY);
        if let Err(error) = platform::send_paste_shortcut() {
            log::warn!("{error}");
        }
    });
    Ok(ActionOutcome::Dismissed)
}

fn run_command(app: &AppHandle, id: &str, language: Language) -> Result<ActionOutcome, String> {
    match id {
        commands::OPEN_SETTINGS => {
            notch::collapse(app, false);
            settings_window::open(app);
        }
        commands::REBUILD_INDEX => {
            indexer::spawn_full_index(app);
            return Ok(done(tr(language, Text::OutcomeRebuildingIndex), false));
        }
        commands::QUIT_APP => app.exit(0),
        commands::LOCK_SCREEN => {
            notch::collapse(app, false);
            platform::lock_workstation()?;
        }
        commands::WINDOWS_SETTINGS => {
            platform::shell_open("ms-settings:", ShellVerb::Open)?;
            notch::collapse(app, false);
        }
        commands::RECYCLE_BIN => {
            platform::shell_open("shell:RecycleBinFolder", ShellVerb::Open)?;
            notch::collapse(app, false);
        }
        unknown => return Err(format!("Unknown command '{unknown}'")),
    }
    Ok(ActionOutcome::Dismissed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, kind: ResultKind) -> SearchResult {
        SearchResult {
            id: path.to_string(),
            kind,
            title: "x".to_string(),
            subtitle: String::new(),
            target: path.to_string(),
            icon_target: None,
            location: Some(path.to_string()),
            pinned: false,
            match_indices: Vec::new(),
        }
    }

    #[test]
    fn terminal_opens_in_the_folder_or_the_file_parent() {
        assert_eq!(working_directory_of(&file("C:\\a\\b", ResultKind::Folder), Language::En).unwrap(), "C:\\a\\b");
        assert_eq!(working_directory_of(&file("C:\\a\\b.txt", ResultKind::File), Language::En).unwrap(), "C:\\a");
    }

    #[test]
    fn results_without_a_file_cannot_be_revealed() {
        let mut store_app = file("shell:AppsFolder\\x", ResultKind::App);
        store_app.location = None;
        assert!(location_of(&store_app, Language::En).is_err());
    }
}
