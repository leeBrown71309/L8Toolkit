import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { useApplyAppearance, useSettingsSnapshot } from "../../shared/appearance";
import { backend, BACKEND_EVENTS } from "../../shared/backend";
import type {
  AccentBarState,
  Material,
  NotchOpenedPayload,
  NowPlaying,
  OpenSource,
  WidgetTab,
} from "../../shared/types";
import { useBackendEvent } from "../../shared/use-backend-event";
import { useElementHeight } from "../../shared/use-element-height";
import { AccentBar, accentBarWidth } from "../accent/accent-bar";
import { useArtworkGlow } from "../music/artwork-glow";
import { MusicPanel } from "../music/music-panel";
import { NowPlayingLive } from "../music/now-playing-live";
import { currentPositionMs, useNowPlaying } from "../music/use-now-playing";
import { SpotlightPanel } from "../spotlight/spotlight-panel";
import { NotchShape, type NotchBackdrop, type NotchLiveActivity, type NotchMode } from "./notch-shape";
import { WidgetHeader } from "./widget-header";

/** Focus can flicker while Windows hands it over right after opening. */
const BLUR_GRACE_MS = 300;
/** A hover-opened notch closes shortly after the pointer leaves it. */
const HOVER_LEAVE_CLOSE_DELAY_MS = 380;
/** How long a new track is shown before the notch folds back. */
const PEEK_DURATION_MS = 3400;
/** A new track peeks if it is confirmed playing within this delay. */
const PEEK_CONFIRM_WINDOW_MS = 5000;
/** Media found during the first seconds after launch does not peek. */
const STARTUP_QUIET_MS = 6000;
/** Notch window is 540px tall; keep room for the drop shadow. */
const MAX_PANEL_HEIGHT = 500;
const INITIAL_PANEL_HEIGHT = 180;
const TAB_ORDER: WidgetTab[] = ["search", "music"];
const TAB_SHORTCUTS: Record<string, WidgetTab> = { "1": "search", "2": "music" };

interface PendingPeek {
  track: string;
  since: number;
}

interface OpenSession {
  id: number;
  source: OpenSource;
}

/** Wallpaper for the Mica material, refreshed at each opening (it may change). */
function useWallpaper(enabled: boolean, sessionId: number): string | null {
  const [wallpaper, setWallpaper] = useState<string | null>(null);
  useEffect(() => {
    if (!enabled) return;
    let alive = true;
    backend
      .getWallpaper()
      .then((image) => {
        if (alive) setWallpaper(image);
      })
      .catch((error: unknown) => console.error("Cannot load the wallpaper", error));
    return () => {
      alive = false;
    };
  }, [enabled, sessionId]);
  return wallpaper;
}

function resolveBackdrop(material: Material | undefined, wallpaper: string | null, snapshot: string | null): NotchBackdrop | null {
  if (material === "mica" && wallpaper) return { image: wallpaper, fit: "screen" };
  // Acrylic, or Mica without a readable wallpaper: what is behind the notch.
  if ((material === "acrylic" || material === "mica") && snapshot) return { image: snapshot, fit: "window" };
  return null;
}

/** Track identity, to notice when the song (or video) changes. */
function trackKey(media: NowPlaying | null): string | null {
  return media ? `${media.sourceName}|${media.title}|${media.artist}` : null;
}

/**
 * Root of the notch window: tracks open/idle/peek state, which widget is
 * shown, and decides when to close (Escape, blur, click outside, hover leave).
 */
export function NotchApp() {
  const [mode, setMode] = useState<NotchMode>("idle");
  const [session, setSession] = useState<OpenSession>({ id: 0, source: "hotkey" });
  const [tab, setTab] = useState<WidgetTab>("search");
  const [tabDirection, setTabDirection] = useState<"forward" | "backward">("forward");
  const [panelHeight, setPanelHeight] = useState(INITIAL_PANEL_HEIGHT);
  const [snapshot, setSnapshot] = useState<string | null>(null);
  const [accent, setAccent] = useState<AccentBarState | null>(null);
  /** Characters of the last open bar: the width to keep while it closes. */
  const accentWidthRef = useRef(1);
  if (accent?.visible && accent.characters.length > 0) accentWidthRef.current = accent.characters.length;
  const { t } = useI18n();
  const panelRef = useRef<HTMLDivElement>(null);

  const settings = useSettingsSnapshot();
  const appearance = settings?.settings.appearance;
  const music = settings?.settings.music;
  const searchEnabled = settings?.settings.spotlight.enabled ?? true;
  const musicEnabled = music?.enabled ?? false;
  useApplyAppearance(appearance, settings?.systemAccentColor ?? null);

  const media = useNowPlaying(musicEnabled);
  const glow = useArtworkGlow(media?.artwork ?? null, Boolean(music?.artworkTint));
  const wallpaper = useWallpaper(appearance?.material === "mica", session.id);
  const backdrop = resolveBackdrop(appearance?.material, wallpaper, snapshot);
  const tabs = useMemo(
    () => TAB_ORDER.filter((candidate) => (candidate === "search" ? searchEnabled : musicEnabled)),
    [searchEnabled, musicEnabled],
  );
  const activeTab = tabs.includes(tab) ? tab : (tabs[0] ?? "search");

  const modeRef = useRef<NotchMode>("idle");
  const sessionRef = useRef(session);
  const mediaRef = useRef(media);
  const tabsRef = useRef(tabs);
  const tabRef = useRef<WidgetTab>(activeTab);
  const openedAtRef = useRef(0);
  const engagedRef = useRef(false);
  const leaveTimerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    modeRef.current = mode;
    sessionRef.current = session;
    mediaRef.current = media;
    tabsRef.current = tabs;
    tabRef.current = activeTab;
  });

  useElementHeight(panelRef, setPanelHeight);

  const cancelLeaveTimer = useCallback(() => {
    window.clearTimeout(leaveTimerRef.current);
    leaveTimerRef.current = undefined;
  }, []);

  const selectTab = useCallback((next: WidgetTab) => {
    setTab((current) => {
      if (current !== next) {
        setTabDirection(TAB_ORDER.indexOf(next) > TAB_ORDER.indexOf(current) ? "forward" : "backward");
      }
      return next;
    });
  }, []);

  const handleOpened = useCallback(
    ({ source, backdrop: screenBehind }: NotchOpenedPayload) => {
      engagedRef.current = false;
      openedAtRef.current = performance.now();
      setSnapshot(screenBehind ?? null);
      setSession((previous) => ({ id: previous.id + 1, source }));
      // Hovering a notch that shows music means "show me the music";
      // the keyboard shortcut means "search".
      const availableTabs = tabsRef.current;
      const wantsMusic = source === "hover" && Boolean(mediaRef.current) && availableTabs.includes("music");
      setTab(wantsMusic || !availableTabs.includes("search") ? "music" : "search");
      setMode("open");
    },
    [],
  );

  const close = useCallback(
    (restoreFocus: boolean) => {
      if (modeRef.current !== "open") return;
      cancelLeaveTimer();
      setMode("idle");
      backend.notchCollapse(restoreFocus).catch((error: unknown) => console.error("Cannot collapse the notch", error));
    },
    [cancelLeaveTimer],
  );

  useBackendEvent<NotchOpenedPayload>(BACKEND_EVENTS.notchOpened, handleOpened);
  useBackendEvent<AccentBarState>(BACKEND_EVENTS.accentState, setAccent);
  useBackendEvent<null>(BACKEND_EVENTS.notchClosed, () => {
    cancelLeaveTimer();
    setMode("idle");
  });

  // The window may have been opened before this listener existed.
  useEffect(() => {
    backend
      .notchState()
      .then((state) => {
        if (state.isOpen) handleOpened({ source: "hotkey" });
      })
      .catch((error: unknown) => console.error("Cannot read the notch state", error));
  }, [handleOpened]);

  useEffect(() => {
    const handleBlur = () => {
      const settled = performance.now() - openedAtRef.current > BLUR_GRACE_MS;
      if (modeRef.current === "open" && settled) close(false);
    };
    window.addEventListener("blur", handleBlur);
    return () => window.removeEventListener("blur", handleBlur);
  }, [close]);

  // Widget switching from anywhere in the panel: Alt+Left / Alt+Right cycle
  // through the tools, Ctrl+1 / Ctrl+2 jump to one. Captured before the
  // widgets so neither the search field nor the music keys swallow it.
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (modeRef.current !== "open") return;
      const available = tabsRef.current;
      const cycle = event.altKey && !event.ctrlKey && (event.key === "ArrowLeft" || event.key === "ArrowRight");
      const direct = event.ctrlKey && !event.altKey ? TAB_SHORTCUTS[event.key] : undefined;
      let next: WidgetTab | undefined;
      if (cycle && available.length > 1) {
        const index = available.indexOf(tabRef.current);
        const step = event.key === "ArrowRight" ? 1 : -1;
        next = available[(index + step + available.length) % available.length];
      } else if (direct && available.includes(direct)) {
        next = direct;
      }
      if (!next) return;
      // Also stops WebView2 from treating Alt+Left as "go back".
      event.preventDefault();
      event.stopPropagation();
      engagedRef.current = true;
      selectTab(next);
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [selectTab]);

  // A new track briefly swells the notch to show it (when enabled). Players
  // bridged from a phone report the new title while still "changing" (not
  // playing yet), so the new track waits a few seconds for playback to be
  // confirmed instead of being judged on that single reading.
  const currentTrack = trackKey(media);
  const isPlaying = Boolean(media?.playing);
  const previousTrackRef = useRef<string | null | undefined>(undefined);
  const pendingPeekRef = useRef<PendingPeek | null>(null);
  const peekTimerRef = useRef<number | undefined>(undefined);
  useEffect(() => {
    const previous = previousTrackRef.current;
    previousTrackRef.current = currentTrack;
    // Media already playing when the app starts is not "a new track".
    const startingUp = performance.now() < STARTUP_QUIET_MS;
    const isNewTrack =
      previous !== undefined && currentTrack !== null && currentTrack !== previous && !(previous === null && startingUp);
    if (isNewTrack) pendingPeekRef.current = { track: currentTrack, since: performance.now() };

    const pending = pendingPeekRef.current;
    if (!pending || pending.track !== currentTrack || !isPlaying) return;
    pendingPeekRef.current = null;
    const fresh = performance.now() - pending.since <= PEEK_CONFIRM_WINDOW_MS;
    const wanted = Boolean(music?.peekOnTrackChange && music.showInNotch);
    if (!fresh || !wanted || modeRef.current === "open") return;
    // Functional update: an open panel always wins over a peek.
    setMode((current) => (current === "open" ? current : "peek"));
    // The timer lives in a ref, not in this effect's cleanup: pausing during
    // the peek must not cancel the fold back.
    window.clearTimeout(peekTimerRef.current);
    peekTimerRef.current = window.setTimeout(
      () => setMode((current) => (current === "peek" ? "idle" : current)),
      PEEK_DURATION_MS,
    );
  }, [currentTrack, isPlaying, music?.peekOnTrackChange, music?.showInNotch]);

  useEffect(() => () => window.clearTimeout(peekTimerRef.current), []);

  useEffect(() => cancelLeaveTimer, [cancelLeaveTimer]);

  const markEngaged = useCallback(() => {
    engagedRef.current = true;
    cancelLeaveTimer();
  }, [cancelLeaveTimer]);

  const handlePanelPointerLeave = useCallback(() => {
    const isPeek = sessionRef.current.source === "hover" && !engagedRef.current;
    if (modeRef.current !== "open" || !isPeek) return;
    cancelLeaveTimer();
    leaveTimerRef.current = window.setTimeout(() => close(true), HOVER_LEAVE_CLOSE_DELAY_MS);
  }, [cancelLeaveTimer, close]);

  const openSettings = useCallback(() => {
    backend.openSettings().catch((error: unknown) => console.error("Cannot open settings", error));
  }, []);

  // Stable: reads the latest media through the ref, so the light's timer is
  // not restarted on every render.
  const readProgress = useCallback(() => {
    const current = mediaRef.current;
    if (!current || current.durationMs <= 0) return null;
    return currentPositionMs(current) / current.durationMs;
  }, []);

  const live: NotchLiveActivity | null =
    media && music?.showInNotch
      ? {
          content: (
            <NowPlayingLive
              media={media}
              mode={mode === "open" || mode === "accent" || accent?.visible ? "open" : mode}
              showAudioBars={Boolean(music.showAudioBars)}
            />
          ),
          playing: media.playing,
          glow,
          readProgress,
        }
      : null;

  // The open panel always wins; otherwise an accent bar takes over the idle
  // notch (or a peek) until it closes.
  const accentVisible = Boolean(accent?.visible) && mode !== "open";
  const shownMode: NotchMode = accentVisible ? "accent" : mode;
  // Once used, the bar stays mounted so it can fade out with the notch.
  const overlay = accent
    ? {
        content: <AccentBar state={mode === "open" ? null : accent} label={t("accent.barLabel")} />,
        width: accentBarWidth(Math.max(1, accentWidthRef.current)),
      }
    : null;

  return (
    <NotchShape
      mode={shownMode}
      openHeight={Math.min(panelHeight, MAX_PANEL_HEIGHT)}
      backdrop={backdrop}
      live={live}
      overlay={overlay}
      onBackdropPointerDown={() => close(true)}
      onPanelPointerEnter={cancelLeaveTimer}
      onPanelPointerLeave={handlePanelPointerLeave}
    >
      <div ref={panelRef} className="widget-panel">
        <WidgetHeader
          tabs={tabs}
          active={activeTab}
          musicPlaying={Boolean(media?.playing)}
          onSelect={(next) => {
            markEngaged();
            selectTab(next);
          }}
          onOpenSettings={openSettings}
        />
        <div className="widget-views" data-direction={tabDirection}>
          {searchEnabled && (
            <div className="widget-view" hidden={activeTab !== "search"}>
              <SpotlightPanel
                key={session.id}
                isOpen={mode === "open"}
                isActive={activeTab === "search"}
                onClose={close}
                onEngaged={markEngaged}
              />
            </div>
          )}
          {musicEnabled && (
            <div className="widget-view" hidden={activeTab !== "music"}>
              <MusicPanel
                media={media}
                isActive={mode === "open" && activeTab === "music"}
                ambient={Boolean(music?.artworkTint)}
                onClose={close}
                onEngaged={markEngaged}
              />
            </div>
          )}
        </div>
      </div>
    </NotchShape>
  );
}
