import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { createPreviewBackend } from "./preview-backend";
import type {
  ActionOutcome,
  AppSettings,
  ExecuteAction,
  NotchSnapshot,
  NowPlaying,
  PlaybackRequest,
  SearchResult,
  SearchSection,
  SettingsSnapshot,
  UpdateState,
  AccentLanguage,
  AccentPreviewRow,
  TimerRequest,
  TimerSnapshot,
} from "./types";

export const BACKEND_EVENTS = {
  notchOpened: "notch:opened",
  notchClosed: "notch:closed",
  settingsChanged: "settings:changed",
  indexStatus: "index:status",
  mediaChanged: "media:changed",
  /** Audio band levels (0-255, low to high frequencies), ~30 per second. */
  audioLevels: "media:levels",
  updateState: "update:state",
  accentState: "accent:state",
  timerState: "timer:state",
  /** A countdown or a Pomodoro phase ended. */
  timerFinished: "timer:finished",
  /** Asks the settings window to show a section (the tray's "Install update"). */
  settingsNavigate: "settings:navigate",
} as const;

export type Unlisten = () => void;

/** Everything the UI needs from the Rust side, behind one typed surface. */
export interface Backend {
  search(query: string): Promise<SearchSection[]>;
  frequentItems(): Promise<SearchResult[]>;
  executeResult(result: SearchResult, action: ExecuteAction): Promise<ActionOutcome>;
  resolveIcon(target: string): Promise<string | null>;
  notchState(): Promise<NotchSnapshot>;
  notchCollapse(restoreFocus: boolean): Promise<void>;
  /** Desktop wallpaper thumbnail (Mica material), or null when unavailable. */
  getWallpaper(): Promise<string | null>;
  mediaState(): Promise<NowPlaying | null>;
  mediaCommand(request: PlaybackRequest): Promise<void>;
  /** Brings the player app to the front and closes the notch. */
  mediaOpenSource(): Promise<void>;
  /** Starts or stops the audio capture behind the notch's audio bars. */
  audioBarsSetVisible(visible: boolean): Promise<void>;
  updateState(): Promise<UpdateState>;
  /** Asks GitHub Releases for a newer version. */
  updateCheck(): Promise<UpdateState>;
  /** Downloads, verifies and installs the newest version; the app restarts. */
  updateInstall(): Promise<void>;
  /** Letters and their accents for these languages (empty: all). */
  quickAccentPreview(languages: AccentLanguage[]): Promise<AccentPreviewRow[]>;
  timerState(): Promise<TimerSnapshot>;
  timerCommand(request: TimerRequest): Promise<TimerSnapshot>;
  /** The folded notch shows a second activity in a bubble (its hover zone). */
  notchSetLayout(bubble: boolean): Promise<void>;
  getSettings(): Promise<SettingsSnapshot>;
  updateSettings(settings: AppSettings): Promise<SettingsSnapshot>;
  rebuildIndex(): Promise<void>;
  openSettings(): Promise<void>;
  /** The settings page is drawn: Rust shows its (hidden) window. */
  settingsWindowReady(): Promise<void>;
  quitApp(): Promise<void>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<Unlisten>;
}

/** True inside the Tauri webview, false in a plain browser (design preview). */
export const isTauriRuntime = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const tauriBackend: Backend = {
  search: (query) => invoke("search", { query }),
  frequentItems: () => invoke("frequent_items"),
  executeResult: (result, action) => invoke("execute_result", { result, action }),
  resolveIcon: (target) => invoke("resolve_icon", { target }),
  notchState: () => invoke("notch_state"),
  notchCollapse: (restoreFocus) => invoke("notch_collapse", { restoreFocus }),
  getWallpaper: () => invoke("get_wallpaper"),
  mediaState: () => invoke("media_state"),
  mediaCommand: (request) => invoke("media_command", { request }),
  mediaOpenSource: () => invoke("media_open_source"),
  audioBarsSetVisible: (visible) => invoke("audio_bars_set_visible", { visible }),
  updateState: () => invoke("update_state"),
  updateCheck: () => invoke("update_check"),
  updateInstall: () => invoke("update_install"),
  quickAccentPreview: (languages) => invoke("quick_accent_preview", { languages }),
  timerState: () => invoke("timer_state"),
  timerCommand: (request) => invoke("timer_command", { request }),
  notchSetLayout: (bubble) => invoke("notch_set_layout", { bubble }),
  getSettings: () => invoke("get_settings"),
  updateSettings: (settings) => invoke("update_settings", { settings }),
  rebuildIndex: () => invoke("rebuild_index"),
  openSettings: () => invoke("open_settings"),
  settingsWindowReady: () => invoke("settings_window_ready"),
  quitApp: () => invoke("quit_app"),
  listen: (event, handler) => listen(event, (message) => handler(message.payload as never)),
};

export const backend: Backend = isTauriRuntime ? tauriBackend : createPreviewBackend();

/** IPC errors arrive as plain strings from Rust; normalise anything else. */
export function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong";
}
