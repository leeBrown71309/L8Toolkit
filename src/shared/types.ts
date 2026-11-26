/** Mirrors the Rust types serialised over IPC (camelCase). */

export type ResultKind = "app" | "file" | "folder" | "calculator" | "web" | "url" | "command";

export interface SearchResult {
  id: string;
  kind: ResultKind;
  title: string;
  subtitle: string;
  target: string;
  iconTarget: string | null;
  /** File or executable on disk; null for Store apps, web rows, commands. */
  location: string | null;
  pinned: boolean;
  matchIndices: number[];
}

export type SectionId = "calculator" | "topHit" | "applications" | "commands" | "folders" | "files" | "web";

export interface SearchSection {
  id: SectionId;
  items: SearchResult[];
}

/** Actions executed by Rust (clipboard-only actions run in the UI). */
export type ExecuteAction =
  | "open"
  | "runAsAdmin"
  | "reveal"
  | "openWith"
  | "openInTerminal"
  | "properties"
  | "copyFile"
  | "moveToRecycleBin"
  | "pin"
  | "unpin"
  | "forget"
  | "manageApps"
  | "pasteClipboard";

export type ActionOutcome = { type: "dismissed" } | { type: "done"; message: string; refresh: boolean };

export type ThemeMode = "system" | "dark" | "light";
export type Material = "solid" | "translucent" | "mica" | "acrylic";
export type AccentColor = "ember" | "ocean" | "jade" | "rose" | "iris" | "graphite" | "system";

export interface AppearanceSettings {
  theme: ThemeMode;
  material: Material;
  accent: AccentColor;
}

export type OpenSource = "hotkey" | "hover" | "tray";

export type WebSearchEngine = "google" | "bing" | "duckduckgo" | "brave";

export interface SpotlightSettings {
  enabled: boolean;
  hotkey: string;
  showIdleNotch: boolean;
  hoverToOpen: boolean;
  webSearchEngine: WebSearchEngine;
  indexFiles: boolean;
  extraFolders: string[];
}

export interface MusicSettings {
  enabled: boolean;
  showInNotch: boolean;
  peekOnTrackChange: boolean;
  artworkTint: boolean;
  /** Bars in the folded notch that follow the sound being played. */
  showAudioBars: boolean;
}

/** Languages whose accents Quick Accent offers (codes stored in the settings). */
export type AccentLanguage = "fr" | "es" | "pt" | "de" | "it" | "nl" | "tr" | "pl" | "ro" | "cs" | "nordic";

/** Keys that open the accent bar and move through it. */
export type AccentTrigger = "spaceAndArrows" | "space" | "arrows";

export interface QuickAccentSettings {
  enabled: boolean;
  /** In priority order; empty means every language. */
  languages: AccentLanguage[];
  trigger: AccentTrigger;
}

/** One letter and its accents, for the settings preview. */
export interface AccentPreviewRow {
  letter: string;
  characters: string[];
}

/** What the accent bar shows, sent by quick_accent/mod.rs. */
export interface AccentBarState {
  visible: boolean;
  characters: string[];
  index: number;
}

export type LanguagePreference = "system" | "en" | "fr";

export interface AppSettings {
  schemaVersion: number;
  language: LanguagePreference;
  launchAtStartup: boolean;
  appearance: AppearanceSettings;
  spotlight: SpotlightSettings;
  music: MusicSettings;
  quickAccent: QuickAccentSettings;
}

export interface AppStatus {
  hotkeyError: string | null;
  appsIndexed: number;
  filesIndexed: number;
  indexingFiles: boolean;
  version: string;
  /** The development copy (`bun run app:dev`), not an installed release. */
  devBuild: boolean;
}

export interface SettingsSnapshot {
  settings: AppSettings;
  status: AppStatus;
  /** Windows accent color as "#rrggbb", when readable. */
  systemAccentColor: string | null;
  /** Language actually used ("system" resolved by Rust against Windows). */
  effectiveLanguage: "en" | "fr";
}

/** What the OS media session reports (Spotify, browsers, players). */
export interface NowPlaying {
  title: string;
  artist: string;
  album: string;
  sourceName: string;
  sourceLaunchTarget: string | null;
  /** Cover art or video thumbnail, as a data URL. */
  artwork: string | null;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  /** Unix time (ms) at which positionMs was read. */
  updatedAtMs: number;
  canToggle: boolean;
  canNext: boolean;
  canPrevious: boolean;
  canSeek: boolean;
}

export type PlaybackRequest =
  | { type: "togglePlayPause" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "seek"; positionMs: number };

export type WidgetTab = "search" | "music";

/** In-app update progress, mirrored from updates.rs. */
export type UpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | { status: "upToDate" }
  | { status: "available"; version: string; notes: string; publishedAtMs: number | null }
  | { status: "downloading"; version: string; downloadedBytes: number; totalBytes: number | null }
  | { status: "installing"; version: string }
  | { status: "failed"; message: string };

export interface NotchSnapshot {
  isOpen: boolean;
}

export interface NotchOpenedPayload {
  source: OpenSource;
  /** Screen behind the notch (Acrylic material), as a PNG data URL. */
  backdrop?: string | null;
}
