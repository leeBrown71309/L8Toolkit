import { createPreviewRecents } from "./preview-recents";
import { createPreviewTimer } from "./preview-timer";
import type { Backend, Unlisten } from "./backend";
import type {
  AppSettings,
  NowPlaying,
  ResultKind,
  SearchResult,
  SearchSection,
  SettingsSnapshot,
  UpdateState,
  AccentPreviewRow,
} from "./types";

/**
 * In-browser stand-in for the Rust backend, used only when the UI runs outside
 * Tauri (`bun run dev` in a browser) so the design can be iterated on quickly.
 * Trigger events from the console: `window.l8Preview.emit("notch:opened", { source: "hotkey" })`.
 * Accent bar: `window.l8Preview.emit("accent:state", { visible: true, characters: ["é", "è", "ê", "ë"], index: 1 })`.
 * Screenshot peek: `window.l8Preview.recents.screenshot()`.
 * Shelf drop: `window.l8Preview.emit("notch:file-drag", { type: "enter" })`, then
 * `window.l8Preview.recents.addFiles(["C:\\Docs\\Plan.pdf"])` and `emit("notch:file-drag", { type: "drop" })`.
 * Timer: `window.l8Preview.timer.apply({ type: "startCountdown", durationMs: 60000 })` (`?timerSpeed=40` runs it faster).
 */

const PREVIEW_APPS = [
  "Visual Studio Code",
  "Spotify",
  "Figma",
  "Google Chrome",
  "Windows Terminal",
  "Notion",
  "Discord",
  "Calculator",
  "Paint",
  "Microsoft Edge",
];

const PREVIEW_FILES = [
  { name: "Quarterly report.pdf", parent: "~\\Documents\\Finance", folder: false },
  { name: "l8_tools_kits", parent: "~\\Desktop\\My_workspace\\dev", folder: true },
  { name: "invoice-2026-11.pdf", parent: "~\\Downloads", folder: false },
  { name: "Photos 2026", parent: "~\\Pictures", folder: true },
];

/** Placeholder cover for the design preview: a plain gradient, no real art. */
const PREVIEW_COVER =
  "data:image/svg+xml," +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1b3a8f"/><stop offset="0.55" stop-color="#3d6fd8"/><stop offset="1" stop-color="#0d1b3d"/></linearGradient></defs><rect width="300" height="300" fill="url(#g)"/><circle cx="190" cy="120" r="54" fill="#9cc2ff" opacity="0.35"/></svg>`,
  );

function previewTrack(): NowPlaying {
  return {
    title: "Stargazing",
    artist: "Myles Smith",
    album: "Stargazing",
    sourceName: "Spotify",
    sourceLaunchTarget: "Spotify",
    artwork: PREVIEW_COVER,
    playing: true,
    positionMs: 84_000,
    durationMs: 238_000,
    updatedAtMs: Date.now(),
    canToggle: true,
    canNext: true,
    canPrevious: true,
    canSeek: true,
  };
}

const DEFAULT_SETTINGS: AppSettings = {
  schemaVersion: 2,
  language: "system",
  launchAtStartup: false,
  pauseInFullscreen: true,
  appearance: { theme: "dark", material: "solid", accent: "ember" },
  spotlight: {
    enabled: true,
    hotkey: "Alt+Space",
    showIdleNotch: true,
    hoverToOpen: true,
    webSearchEngine: "google",
    indexFiles: true,
    extraFolders: ["D:\\Projects"],
  },
  music: { enabled: true, showInNotch: true, peekOnTrackChange: true, artworkTint: true, showAudioBars: true },
  quickAccent: { enabled: true, languages: ["fr"], trigger: "spaceAndArrows" },
  timer: {
    enabled: true,
    showInNotch: true,
    notify: true,
    focusMinutes: 25,
    shortBreakMinutes: 5,
    longBreakMinutes: 15,
    roundsBeforeLongBreak: 4,
    autoContinue: true,
  },
  recents: { enabled: true, limit: 10, screenshotPeek: true, keepPasswordManagerItems: true },
};

function previewResult(kind: ResultKind, title: string, subtitle: string, matchIndices: number[] = []): SearchResult {
  const onDisk = kind === "app" || kind === "file" || kind === "folder";
  return {
    id: `${kind}:${title}`,
    kind,
    title,
    subtitle,
    target: title,
    iconTarget: null,
    location: onDisk ? `C:\\Preview\\${title}` : null,
    pinned: title === "Spotify",
    matchIndices,
  };
}

function fuzzyIndices(text: string, query: string): number[] | null {
  const lowerText = text.toLowerCase();
  const indices: number[] = [];
  let cursor = 0;
  for (const character of query.toLowerCase()) {
    if (character === " ") continue;
    const found = lowerText.indexOf(character, cursor);
    if (found === -1) return null;
    indices.push(found);
    cursor = found + 1;
  }
  return indices;
}

/** "a op b" only: the real calculator lives in Rust. */
function evaluateSimpleExpression(expression: string): number | null {
  const match = /^(-?\d+(?:\.\d+)?)\s*([+\-*/])\s*(-?\d+(?:\.\d+)?)$/.exec(expression);
  if (!match) return null;
  const [, left, operator, right] = match;
  const a = Number(left);
  const b = Number(right);
  const operations: Record<string, number> = { "+": a + b, "-": a - b, "*": a * b, "/": b === 0 ? NaN : a / b };
  const result = operations[operator ?? ""];
  return result !== undefined && Number.isFinite(result) ? result : null;
}

function previewSearch(query: string): SearchSection[] {
  const trimmed = query.trim();
  if (!trimmed) return [];
  const sections: SearchSection[] = [];

  const value = evaluateSimpleExpression(trimmed);
  if (value !== null) {
    sections.push({ id: "calculator", items: [previewResult("calculator", String(Number(value.toFixed(10))), `${trimmed} =`)] });
  }

  const apps = PREVIEW_APPS.flatMap((name) => {
    const indices = fuzzyIndices(name, trimmed);
    return indices ? [previewResult("app", name, "Application", indices)] : [];
  });
  const files = trimmed.length < 2 ? [] : PREVIEW_FILES.flatMap((file) => {
    const indices = fuzzyIndices(file.name, trimmed);
    return indices ? [previewResult(file.folder ? "folder" : "file", file.name, file.parent, indices)] : [];
  });

  const [topHit, ...otherApps] = apps;
  if (topHit) sections.push({ id: "topHit", items: [topHit] });
  if (otherApps.length) sections.push({ id: "applications", items: otherApps.slice(0, 5) });
  const folders = files.filter((file) => file.kind === "folder");
  const documents = files.filter((file) => file.kind === "file");
  if (folders.length) sections.push({ id: "folders", items: folders });
  if (documents.length) sections.push({ id: "files", items: documents });
  sections.push({ id: "web", items: [previewResult("web", trimmed, "Search with Google")] });
  return sections;
}

const PREVIEW_AUDIO_BANDS = 15;

const PREVIEW_AVAILABLE_UPDATE: UpdateState = {
  status: "available",
  version: "0.2.0",
  notes: "Audio bars in the folded notch follow the sound.\nA placeholder cover when the player sends none.",
  publishedAtMs: Date.parse("2026-11-26T10:00:00Z"),
};

function previewUpdateState(): UpdateState {
  const requested = new URLSearchParams(location.search).get("update");
  if (requested === "available") return PREVIEW_AVAILABLE_UPDATE;
  if (requested === "failed") return { status: "failed", message: "Cannot check for updates. Check your internet connection." };
  if (requested === "upToDate") return { status: "upToDate" };
  return { status: "idle" };
}

export function createPreviewBackend(): Backend {
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  let snapshot: SettingsSnapshot = {
    settings: DEFAULT_SETTINGS,
    status: { hotkeyError: null, appsIndexed: 214, filesIndexed: 48_213, indexingFiles: false, version: "0.2.0", devBuild: false },
    systemAccentColor: "#0078d4",
    effectiveLanguage: new URLSearchParams(location.search).get("lang") === "fr" ? "fr" : "en",
  };

  let media: NowPlaying | null = previewTrack();

  const emit = (event: string, payload: unknown) => {
    listeners.get(event)?.forEach((handler) => handler(payload));
  };
  const setMedia = (next: NowPlaying | null) => {
    media = next;
    emit("media:changed", media);
  };

  // Fake audio for the bars: a few drifting sines, louder in the bass.
  let audioTimer: number | undefined;
  // `l8Preview.setRemoteAudio(true)`: media "plays" on a phone, the PC hears nothing.
  let remoteAudio = false;
  const setRemoteAudio = (remote: boolean) => {
    remoteAudio = remote;
  };
  const setAudioBars = (visible: boolean) => {
    window.clearInterval(audioTimer);
    audioTimer = undefined;
    if (!visible) {
      emit("media:levels", new Array(PREVIEW_AUDIO_BANDS).fill(0));
      return;
    }
    audioTimer = window.setInterval(() => {
      if (remoteAudio) {
        emit("media:levels", new Array(PREVIEW_AUDIO_BANDS).fill(0));
        return;
      }
      const time = performance.now() / 1000;
      const beat = Math.max(0, Math.sin(time * Math.PI * 2 * 1.9)) ** 6;
      const levels = Array.from({ length: PREVIEW_AUDIO_BANDS }, (_, band) => {
        const wave = 0.5 + 0.5 * Math.sin(time * (2.3 + band * 0.7) + band * 1.3);
        const weight = 1 - band / (PREVIEW_AUDIO_BANDS * 1.4);
        const level = (0.25 + 0.55 * wave + 0.35 * beat * (band < 4 ? 1 : 0.3)) * weight;
        return Math.round(Math.min(1, level) * 255);
      });
      emit("media:levels", levels);
    }, 33);
  };

  // `?update=available|failed|upToDate` opens the settings on that state.
  let updateState: UpdateState = previewUpdateState();
  const setUpdateState = (next: UpdateState) => {
    updateState = next;
    emit("update:state", next);
  };

  const timer = createPreviewTimer(() => snapshot.settings.timer, emit);
  const recents = createPreviewRecents(emit);

  (window as unknown as { l8Preview: unknown }).l8Preview = { emit, setMedia, previewTrack, setRemoteAudio, setUpdateState, timer, recents };

  return {
    search: async (query) => previewSearch(query),
    frequentItems: async () => PREVIEW_APPS.slice(0, 6).map((name) => previewResult("app", name, "Application")),
    executeResult: async (result, action) => {
      console.info("[preview] execute", action, result.title);
      return { type: "done", message: `Preview: ${action}`, refresh: false };
    },
    resolveIcon: async () => null,
    notchState: async () => ({ isOpen: new URLSearchParams(location.search).get("state") === "open" }),
    notchCollapse: async () => emit("notch:closed", null),
    getWallpaper: async () => null,
    mediaState: async () => media,
    mediaCommand: async (request) => {
      if (!media) return;
      const now = Date.now();
      const position = media.playing ? media.positionMs + (now - media.updatedAtMs) : media.positionMs;
      if (request.type === "togglePlayPause") setMedia({ ...media, playing: !media.playing, positionMs: position, updatedAtMs: now });
      if (request.type === "seek") setMedia({ ...media, positionMs: request.positionMs, updatedAtMs: now });
      if (request.type === "next" || request.type === "previous") setMedia({ ...media, positionMs: 0, updatedAtMs: now });
    },
    mediaOpenSource: async () => console.info("[preview] open player"),
    audioBarsSetVisible: async (visible) => setAudioBars(visible),
    quickAccentPreview: async (languages) => {
      const french: AccentPreviewRow[] = [
        { letter: "a", characters: ["à", "â", "æ", "ä"] },
        { letter: "c", characters: ["ç"] },
        { letter: "e", characters: ["é", "è", "ê", "ë"] },
        { letter: "i", characters: ["î", "ï"] },
        { letter: "o", characters: ["ô", "œ", "ö"] },
        { letter: "u", characters: ["ù", "û", "ü"] },
        { letter: "y", characters: ["ÿ"] },
      ];
      return languages.length === 1 && languages[0] === "fr" ? french : [...french, { letter: "n", characters: ["ñ", "ń", "ň"] }];
    },
    updateState: async () => updateState,
    updateCheck: async () => {
      setUpdateState({ status: "checking" });
      await new Promise((resolve) => window.setTimeout(resolve, 700));
      setUpdateState(PREVIEW_AVAILABLE_UPDATE);
      return updateState;
    },
    updateInstall: async () => {
      const totalBytes = 9_600_000;
      for (let downloadedBytes = 0; downloadedBytes <= totalBytes; downloadedBytes += totalBytes / 40) {
        setUpdateState({ status: "downloading", version: "0.2.0", downloadedBytes, totalBytes });
        await new Promise((resolve) => window.setTimeout(resolve, 80));
      }
      setUpdateState({ status: "installing", version: "0.2.0" });
    },
    timerState: async () => timer.snapshot(),
    timerCommand: async (request) => timer.apply(request),
    notchSetLayout: async () => undefined,
    recentsList: async () => recents.list(),
    recentsCopy: async (id) => recents.copy(id),
    recentsOpen: async (id) => console.info("[preview] open recent", id),
    recentsDrag: async (id) => console.info("[preview] drag recent", id),
    recentsRemove: async (id) => recents.remove(id),
    recentsClear: async () => recents.clear(),
    getSettings: async () => snapshot,
    updateSettings: async (settings) => {
      snapshot = { ...snapshot, settings };
      emit("settings:changed", snapshot);
      return snapshot;
    },
    rebuildIndex: async () => console.info("[preview] rebuild index"),
    openSettings: async () => console.info("[preview] open settings"),
    settingsWindowReady: async () => undefined,
    quitApp: async () => console.info("[preview] quit"),
    listen: async <T,>(event: string, handler: (payload: T) => void): Promise<Unlisten> => {
      const handlers = listeners.get(event) ?? new Set();
      const wrapped = handler as (payload: unknown) => void;
      handlers.add(wrapped);
      listeners.set(event, handlers);
      return () => handlers.delete(wrapped);
    },
  };
}
