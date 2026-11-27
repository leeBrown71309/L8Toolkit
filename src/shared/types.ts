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

export type OpenSource = "hotkey" | "hover" | "tray" | "drop";

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

/** Character sets Quick Accent offers (codes stored in the settings, see characters.rs). */
export type AccentLanguage =
  | "fr" | "es" | "pt" | "de" | "it" | "nl" | "ca" | "tr" | "pl" | "ro" | "cs" | "sk" | "hu" | "hr" | "sl"
  | "nordic" | "fi" | "is" | "et" | "lv" | "lt" | "mt" | "cy" | "ga" | "sq" | "ku" | "eo" | "vi" | "mi"
  | "pinyin" | "currency" | "symbols";

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

export interface TimerSettings {
  enabled: boolean;
  /** The running timer in the folded notch. */
  showInNotch: boolean;
  /** Windows notification when a period ends. */
  notify: boolean;
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  roundsBeforeLongBreak: number;
  /** The next Pomodoro phase starts by itself. */
  autoContinue: boolean;
}

export interface RecentsSettings {
  enabled: boolean;
  /** How many items are kept. */
  limit: number;
  /** The notch briefly shows a new screenshot. */
  screenshotPeek: boolean;
  /** Copies from password managers are kept (never shown in clear). */
  keepPasswordManagerItems: boolean;
}

export type LanguagePreference = "system" | "en" | "fr";

export interface AppSettings {
  schemaVersion: number;
  language: LanguagePreference;
  launchAtStartup: boolean;
  /** While a fullscreen app has the focus, the search shortcut and Quick Accent let every key through. */
  pauseInFullscreen: boolean;
  appearance: AppearanceSettings;
  spotlight: SpotlightSettings;
  music: MusicSettings;
  quickAccent: QuickAccentSettings;
  timer: TimerSettings;
  recents: RecentsSettings;
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

export type WidgetTab = "search" | "music" | "timer" | "recents";

export type RecentKind = "text" | "image" | "files";
/** copy: Ctrl+C anywhere; screenshot: a screen capture; shelf: dropped on the notch. */
export type RecentOrigin = "copy" | "screenshot" | "shelf";

export interface RecentFile {
  path: string;
  name: string;
  /** False once the file was moved or deleted. */
  exists: boolean;
}

/** One Recents item, mirrored from recents/mod.rs. */
export interface RecentView {
  id: string;
  kind: RecentKind;
  origin: RecentOrigin;
  createdAtMs: number;
  /** The beginning of a text; null for sensitive items. */
  preview: string | null;
  textLength: number;
  files: RecentFile[];
  /** Small PNG data URL for images. */
  thumbnail: string | null;
  imageSize: [number, number] | null;
  /** Copied from a password manager: never shown in clear. */
  sensitive: boolean;
  sourceApp: string | null;
}

/** Files dragged over the notch window (Tauri drag and drop). */
/** Files dragged over the notch (Rust keeps the dropped ones itself). */
export type FileDragEvent = { type: "enter" | "leave" | "drop" };

export type TimerMode = "countdown" | "pomodoro";
export type PomodoroPhase = "focus" | "shortBreak" | "longBreak";
export type TimerStatus = "idle" | "running" | "paused";

/** Mirrors timer/machine.rs. While running, time left is derived from `endsAtMs`. */
export interface TimerSnapshot {
  mode: TimerMode;
  status: TimerStatus;
  phase: PomodoroPhase | null;
  durationMs: number;
  remainingMs: number;
  /** Unix time (ms) at which the period ends, while running. */
  endsAtMs: number | null;
  completedFocus: number;
  rounds: number;
  updatedAtMs: number;
}

/** A period came to its end (timer/machine.rs). */
export interface TimerFinished {
  mode: TimerMode;
  phase: PomodoroPhase | null;
  nextPhase: PomodoroPhase | null;
  continued: boolean;
}

export type TimerRequest =
  | { type: "startCountdown"; durationMs: number }
  | { type: "startPomodoro" }
  | { type: "pause" }
  | { type: "resume" }
  | { type: "reset" }
  | { type: "addTime"; deltaMs: number }
  | { type: "skip" };

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

/** Where the pointer rested: the notch, or the bubble of a second activity beside it. */
export type HoverZone = "notch" | "bubble";

export interface NotchOpenedPayload {
  source: OpenSource;
  zone?: HoverZone;
  /** Screen behind the notch (Acrylic material), as a PNG data URL. */
  backdrop?: string | null;
}
