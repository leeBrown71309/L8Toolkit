//! Built-in commands reachable from the search field.

use crate::i18n::Text;

pub const OPEN_SETTINGS: &str = "l8.settings";
pub const REBUILD_INDEX: &str = "l8.reindex";
pub const QUIT_APP: &str = "l8.quit";
pub const LOCK_SCREEN: &str = "system.lock";
pub const WINDOWS_SETTINGS: &str = "system.settings";
pub const RECYCLE_BIN: &str = "system.recycle-bin";

pub struct SystemAction {
    pub id: &'static str,
    pub title: Text,
    pub subtitle: Text,
    /// Extra words the action answers to ("preferences" finds Settings).
    pub keywords: Text,
}

pub const SYSTEM_ACTIONS: &[SystemAction] = &[
    SystemAction {
        id: OPEN_SETTINGS,
        title: Text::CommandSettingsTitle,
        subtitle: Text::CommandSettingsSubtitle,
        keywords: Text::CommandSettingsKeywords,
    },
    SystemAction {
        id: REBUILD_INDEX,
        title: Text::CommandReindexTitle,
        subtitle: Text::CommandReindexSubtitle,
        keywords: Text::CommandReindexKeywords,
    },
    SystemAction {
        id: QUIT_APP,
        title: Text::CommandQuitTitle,
        subtitle: Text::CommandQuitSubtitle,
        keywords: Text::CommandQuitKeywords,
    },
    SystemAction {
        id: LOCK_SCREEN,
        title: Text::CommandLockTitle,
        subtitle: Text::CommandLockSubtitle,
        keywords: Text::CommandLockKeywords,
    },
    SystemAction {
        id: WINDOWS_SETTINGS,
        title: Text::CommandWindowsSettingsTitle,
        subtitle: Text::CommandWindowsSettingsSubtitle,
        keywords: Text::CommandWindowsSettingsKeywords,
    },
    SystemAction {
        id: RECYCLE_BIN,
        title: Text::CommandRecycleBinTitle,
        subtitle: Text::CommandRecycleBinSubtitle,
        keywords: Text::CommandRecycleBinKeywords,
    },
];

/// Actions whose launch is worth remembering for the "Frequent" row.
pub fn is_trackable(id: &str) -> bool {
    !matches!(id, QUIT_APP | REBUILD_INDEX)
}
