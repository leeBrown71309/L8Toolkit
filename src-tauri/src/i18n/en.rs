//! English texts (reference language).

use super::Text;

pub fn text(text: Text) -> &'static str {
    match text {
        Text::ResultApplication => "Application",
        Text::ResultStoreApp => "Store app",
        Text::ResultOpenInBrowser => "Open in your browser",
        Text::ResultSearchWith => "Search with {engine}",

        Text::CommandSettingsTitle => "L8 Settings",
        Text::CommandSettingsSubtitle => "Shortcut, widgets and appearance",
        Text::CommandSettingsKeywords => "preferences configuration options hotkey shortcut",
        Text::CommandReindexTitle => "Rebuild Search Index",
        Text::CommandReindexSubtitle => "Rescan apps and files",
        Text::CommandReindexKeywords => "refresh reindex scan update",
        Text::CommandQuitTitle => "Quit L8 Toolkit",
        Text::CommandQuitSubtitle => "Close every widget",
        Text::CommandQuitKeywords => "exit close stop",
        Text::CommandLockTitle => "Lock Screen",
        Text::CommandLockSubtitle => "Lock this Windows session",
        Text::CommandLockKeywords => "lock session away",
        Text::CommandWindowsSettingsTitle => "Windows Settings",
        Text::CommandWindowsSettingsSubtitle => "System settings",
        Text::CommandWindowsSettingsKeywords => "control panel system preferences",
        Text::CommandRecycleBinTitle => "Recycle Bin",
        Text::CommandRecycleBinSubtitle => "Deleted files",
        Text::CommandRecycleBinKeywords => "trash deleted bin",

        Text::OutcomeFileCopied => "Copied, paste it in any folder",
        Text::OutcomeMovedToRecycleBin => "Moved to the Recycle Bin",
        Text::OutcomePinned => "Pinned to Frequent",
        Text::OutcomeUnpinned => "Unpinned",
        Text::OutcomeRemovedFromFrequent => "Removed from Frequent",
        Text::OutcomeRebuildingIndex => "Rebuilding the index",
        Text::OutcomeCopied => "Copied",

        Text::ErrorOnlyProgramsElevate => "Only programs can run as administrator",
        Text::ErrorOpenWithFilesOnly => "Open with is only available for files",
        Text::ErrorOnlyFilesDeletable => "Only files and folders can be deleted",
        Text::ErrorOnlyPinnable => "Only apps, files and folders can be pinned",
        Text::ErrorNoFileOnDisk => "{title} has no file on disk",
        Text::ErrorNothingPlaying => "Nothing is playing",
        Text::ErrorPlayerRefused => "The player did not accept this action",
        Text::ErrorPlayerUnknown => "The player app is not known",
        Text::ErrorMediaUnavailable => "Media controls are not available",
        Text::ErrorShortcutInUse => "{shortcut} is already used by another application",
        Text::ErrorShortcutInvalid => "{shortcut} is not a valid shortcut",

        Text::TrayOpenSearch => "Open search",
        Text::TraySettings => "Settings…",
        Text::TrayInstallUpdate => "Install update {version}…",
        Text::TrayQuit => "Quit L8 Toolkit",

        Text::UpdateCheckFailed => "Cannot check for updates. Check your internet connection and try again.",
        Text::UpdateInstallFailed => "The update could not be installed. Try again in a moment.",
        Text::UpdateBusy => "An update check is already running",
        Text::UpdateNotificationTitle => "L8 Toolkit {version} is available",
        Text::UpdateNotificationBody => "Install it in one click from Settings › Updates, or from the tray icon.",

        Text::TimerDoneTitle => "Time's up",
        Text::TimerDoneBody => "Your timer has finished.",
        Text::PomodoroFocusDoneTitle => "Focus session complete",
        Text::PomodoroBreakDoneTitle => "Break is over",
        Text::PomodoroFocusStarted => "Back to work: {minutes} minutes of focus have started.",
        Text::PomodoroFocusReady => "Start the next {minutes}-minute focus session from the notch.",
        Text::PomodoroBreakStarted => "Take a {minutes}-minute break, it has started.",
        Text::PomodoroBreakReady => "Start your {minutes}-minute break from the notch.",

        Text::RecentsScreenshotName => "Screenshot",
        Text::RecentsImageName => "Image",
        Text::ErrorRecentGone => "This item is no longer available",
        Text::ErrorRecentFilesMissing => "These files were moved or deleted",
    }
}
