//! User preferences, persisted as JSON in the app config directory.

use std::path::PathBuf;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::i18n::{Language, LanguagePreference};
use crate::json_file;
use crate::platform;
use crate::quick_accent::{AccentLanguage, AccentTrigger};

pub const DEFAULT_SPOTLIGHT_HOTKEY: &str = "Alt+Space";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchEngine {
    #[default]
    Google,
    Bing,
    DuckDuckGo,
    Brave,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SpotlightSettings {
    pub enabled: bool,
    pub hotkey: String,
    /// Keep the small notch visible at the top of the screen while idle.
    pub show_idle_notch: bool,
    /// Expand the notch when the pointer rests on it.
    pub hover_to_open: bool,
    pub web_search_engine: WebSearchEngine,
    pub index_files: bool,
    /// Folders indexed in addition to the standard user folders.
    pub extra_folders: Vec<String>,
}

impl Default for SpotlightSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            hotkey: DEFAULT_SPOTLIGHT_HOTKEY.to_string(),
            show_idle_notch: true,
            hover_to_open: true,
            web_search_engine: WebSearchEngine::Google,
            index_files: true,
            extra_folders: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MusicSettings {
    pub enabled: bool,
    /// Live activity in the idle notch (progress light, peek on new tracks).
    pub show_in_notch: bool,
    /// Briefly expand the notch with the new track when it changes.
    pub peek_on_track_change: bool,
    /// Tint the glow and background with the cover's colors.
    pub artwork_tint: bool,
    /// Bars in the folded notch that follow the sound being played.
    pub show_audio_bars: bool,
}

impl Default for MusicSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_in_notch: true,
            peek_on_track_change: false,
            artwork_tint: true,
            show_audio_bars: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    #[default]
    Dark,
    Light,
}

/// What the open panel is made of. The idle notch always stays black.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    #[default]
    Solid,
    /// See-through tint, no blur (pure CSS).
    Translucent,
    /// Mica: the wallpaper, blurred and tinted.
    Mica,
    /// Frosted blur of whatever is behind the panel when it opens.
    Acrylic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccentColor {
    #[default]
    Ember,
    Ocean,
    Jade,
    Rose,
    Iris,
    Graphite,
    /// Follows the Windows accent color.
    System,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppearanceSettings {
    pub theme: ThemeMode,
    pub material: Material,
    pub accent: AccentColor,
}

/// Bumped when stored settings need a migration (see `migrate`).
pub const SETTINGS_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// Missing in files written before versioning, hence 0 for those.
    #[serde(default)]
    pub schema_version: u32,
    pub language: LanguagePreference,
    pub launch_at_startup: bool,
    /// While a fullscreen app (game, video) has the focus, the search
    /// shortcut and Quick Accent let every key through to it.
    pub pause_in_fullscreen: bool,
    pub appearance: AppearanceSettings,
    pub spotlight: SpotlightSettings,
    pub music: MusicSettings,
    pub quick_accent: QuickAccentSettings,
    pub timer: TimerSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TimerSettings {
    pub enabled: bool,
    /// The running timer in the folded notch (light along the edge, time left).
    pub show_in_notch: bool,
    /// Windows notification when a period ends.
    pub notify: bool,
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    /// Focus sessions before a long break.
    pub rounds_before_long_break: u32,
    /// The next Pomodoro phase starts by itself.
    pub auto_continue: bool,
}

impl Default for TimerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_in_notch: true,
            notify: true,
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            rounds_before_long_break: 4,
            auto_continue: true,
        }
    }
}

impl TimerSettings {
    /// Keeps hand-edited files within sensible bounds.
    pub fn sanitized(mut self) -> Self {
        self.focus_minutes = self.focus_minutes.clamp(1, 180);
        self.short_break_minutes = self.short_break_minutes.clamp(1, 60);
        self.long_break_minutes = self.long_break_minutes.clamp(1, 120);
        self.rounds_before_long_break = self.rounds_before_long_break.clamp(1, 12);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuickAccentSettings {
    pub enabled: bool,
    /// Languages whose accents are offered, in priority order. Empty: all.
    pub languages: Vec<AccentLanguage>,
    pub trigger: AccentTrigger,
}

impl Default for QuickAccentSettings {
    /// The Windows display language when it has accents, otherwise all.
    fn default() -> Self {
        let system = platform::system_primary_language().and_then(AccentLanguage::from_windows_primary_language);
        Self { enabled: true, languages: system.into_iter().collect(), trigger: AccentTrigger::default() }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            language: LanguagePreference::System,
            launch_at_startup: false,
            pause_in_fullscreen: true,
            appearance: AppearanceSettings::default(),
            spotlight: SpotlightSettings::default(),
            music: MusicSettings::default(),
            quick_accent: QuickAccentSettings::default(),
            timer: TimerSettings::default(),
        }
    }
}

/// Upgrades settings written by older versions. Returns true when changed.
pub fn migrate(settings: &mut AppSettings) -> bool {
    if settings.schema_version >= SETTINGS_SCHEMA_VERSION {
        return false;
    }
    if settings.schema_version < 2 {
        // Version 1 stored `music.enabled: false` as a placeholder while the
        // widget did not exist; the user never chose it.
        settings.music.enabled = true;
    }
    settings.schema_version = SETTINGS_SCHEMA_VERSION;
    true
}

/// Thread-safe owner of the current settings and their file.
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<AppSettings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let mut current = json_file::read_or_default::<AppSettings>(&path);
        if migrate(&mut current) {
            if let Err(error) = json_file::write_atomically(&path, &current) {
                log::warn!("Cannot save migrated settings: {error}");
            }
        }
        Self { path, current: RwLock::new(current) }
    }

    /// Language actually used for texts produced on the Rust side.
    pub fn language(&self) -> Language {
        Language::resolve(self.read(|settings| settings.language))
    }

    pub fn get(&self) -> AppSettings {
        self.current.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// Reads a single value without cloning the whole document (used by the
    /// pointer watcher, which runs 20 times per second).
    pub fn read<T>(&self, reader: impl FnOnce(&AppSettings) -> T) -> T {
        reader(&self.current.read().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    pub fn replace(&self, next: AppSettings) -> Result<(), String> {
        json_file::write_atomically(&self.path, &next)?;
        *self.current.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_documents_fill_missing_fields_with_defaults() {
        let parsed: AppSettings =
            serde_json::from_str(r#"{ "spotlight": { "hotkey": "Ctrl+K" } }"#).unwrap();
        assert_eq!(parsed.spotlight.hotkey, "Ctrl+K");
        assert!(parsed.spotlight.enabled);
        assert!(parsed.spotlight.show_idle_notch);
        assert!(parsed.music.enabled);
        assert!(parsed.pause_in_fullscreen, "files written before the setting pause in fullscreen");
    }

    #[test]
    fn version_one_files_get_the_music_widget_enabled() {
        let mut legacy: AppSettings = serde_json::from_str(r#"{ "music": { "enabled": false } }"#).unwrap();
        assert_eq!(legacy.schema_version, 0);
        assert!(migrate(&mut legacy));
        assert!(legacy.music.enabled);
        assert_eq!(legacy.schema_version, SETTINGS_SCHEMA_VERSION);

        let mut current = AppSettings { music: MusicSettings { enabled: false, ..MusicSettings::default() }, ..AppSettings::default() };
        assert!(!migrate(&mut current));
        assert!(!current.music.enabled, "a choice made after migration is kept");
    }

    #[test]
    fn appearance_defaults_to_the_original_obsidian_look() {
        let parsed: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.appearance.theme, ThemeMode::Dark);
        assert_eq!(parsed.appearance.material, Material::Solid);
        assert_eq!(parsed.appearance.accent, AccentColor::Ember);
    }

    #[test]
    fn search_engine_uses_lowercase_names() {
        let json = serde_json::to_string(&WebSearchEngine::DuckDuckGo).unwrap();
        assert_eq!(json, "\"duckduckgo\"");
    }
}
