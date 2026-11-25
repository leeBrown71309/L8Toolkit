//! Texts produced on the Rust side: search results, built-in commands,
//! action outcomes, user-facing errors and the tray menu.
//!
//! Every text is a `Text` variant and each language is an exhaustive `match`,
//! so a missing translation does not compile. Low-level diagnostic errors
//! (OS failures carrying a system message) stay in English on purpose.

mod en;
mod fr;

use serde::{Deserialize, Serialize};

use crate::platform;

/// What the user picked in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    #[default]
    System,
    En,
    Fr,
}

/// The language actually used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Fr,
}

/// Windows primary language identifier for French (LANG_FRENCH).
const PRIMARY_LANGUAGE_FRENCH: u16 = 0x0C;

impl Language {
    pub fn resolve(preference: LanguagePreference) -> Self {
        match preference {
            LanguagePreference::En => Language::En,
            LanguagePreference::Fr => Language::Fr,
            LanguagePreference::System => match platform::system_primary_language() {
                Some(PRIMARY_LANGUAGE_FRENCH) => Language::Fr,
                _ => Language::En,
            },
        }
    }
}

macro_rules! texts {
    ($($name:ident),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text {
            $($name),*
        }

        impl Text {
            #[cfg(test)]
            pub const ALL: &'static [Text] = &[$(Text::$name),*];
        }
    };
}

texts! {
    ResultApplication,
    ResultStoreApp,
    ResultOpenInBrowser,
    ResultSearchWith,

    CommandSettingsTitle,
    CommandSettingsSubtitle,
    CommandSettingsKeywords,
    CommandReindexTitle,
    CommandReindexSubtitle,
    CommandReindexKeywords,
    CommandQuitTitle,
    CommandQuitSubtitle,
    CommandQuitKeywords,
    CommandLockTitle,
    CommandLockSubtitle,
    CommandLockKeywords,
    CommandWindowsSettingsTitle,
    CommandWindowsSettingsSubtitle,
    CommandWindowsSettingsKeywords,
    CommandRecycleBinTitle,
    CommandRecycleBinSubtitle,
    CommandRecycleBinKeywords,

    OutcomeFileCopied,
    OutcomeMovedToRecycleBin,
    OutcomePinned,
    OutcomeUnpinned,
    OutcomeRemovedFromFrequent,
    OutcomeRebuildingIndex,
    OutcomeCopied,

    ErrorOnlyProgramsElevate,
    ErrorOpenWithFilesOnly,
    ErrorOnlyFilesDeletable,
    ErrorOnlyPinnable,
    ErrorNoFileOnDisk,
    ErrorNothingPlaying,
    ErrorPlayerRefused,
    ErrorPlayerUnknown,
    ErrorMediaUnavailable,
    ErrorShortcutInUse,
    ErrorShortcutInvalid,

    TrayOpenSearch,
    TraySettings,
    TrayInstallUpdate,
    TrayQuit,

    UpdateCheckFailed,
    UpdateInstallFailed,
    UpdateBusy,
    UpdateNotificationTitle,
    UpdateNotificationBody,
}

pub fn tr(language: Language, text: Text) -> &'static str {
    match language {
        Language::En => en::text(text),
        Language::Fr => fr::text(text),
    }
}

/// Fills `{name}` placeholders.
pub fn tr_with(language: Language, text: Text, arguments: &[(&str, &str)]) -> String {
    arguments
        .iter()
        .fold(tr(language, text).to_string(), |message, (name, value)| message.replace(&format!("{{{name}}}"), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(message: &str) -> Vec<&str> {
        let mut names: Vec<&str> = message
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        names.sort_unstable();
        names
    }

    #[test]
    fn every_text_is_translated_with_the_same_placeholders() {
        for &text in Text::ALL {
            let english = tr(Language::En, text);
            let french = tr(Language::Fr, text);
            assert!(!english.is_empty() && !french.is_empty(), "{text:?} is empty");
            assert_eq!(placeholders(english), placeholders(french), "{text:?} placeholders differ");
        }
    }

    #[test]
    fn placeholders_are_filled() {
        let message = tr_with(Language::Fr, Text::ResultSearchWith, &[("engine", "Google")]);
        assert_eq!(message, "Rechercher avec Google");
    }

    #[test]
    fn explicit_preferences_win_over_the_system() {
        assert_eq!(Language::resolve(LanguagePreference::Fr), Language::Fr);
        assert_eq!(Language::resolve(LanguagePreference::En), Language::En);
    }
}
