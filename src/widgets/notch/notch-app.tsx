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
  TimerFinished,
  WidgetTab,
} from "../../shared/types";
import { useBackendEvent } from "../../shared/use-backend-event";
import { useElementHeight } from "../../shared/use-element-height";
import { AccentBar, accentBarWidth } from "../accent/accent-bar";
import { useArtworkGlow } from "../music/artwork-glow";
import { MusicPanel } from "../music/music-panel";
import { MusicBubbleContent, NowPlayingLive } from "../music/now-playing-live";
import { currentPositionMs, useNowPlaying } from "../music/use-now-playing";
import { SpotlightPanel } from "../spotlight/spotlight-panel";
import { TimerBubbleContent, TimerLive } from "../timer/timer-live";
import { TimerPanel } from "../timer/timer-panel";
import { useTimer } from "../timer/timer-state";
import { elapsedRatio, isTimerActive, timerColor } from "../timer/timer-time";
import { NotchShape, type NotchBackdrop, type NotchBubble, type NotchLiveActivity, type NotchMode } from "./notch-shape";
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
/** How long the notch shows that a timer period ended. */
const TIMER_ALERT_MS = 5000;
const TAB_ORDER: WidgetTab[] = ["search", "music", "timer"];
const TAB_SHORTCUTS: Record<string, WidgetTab> = { "1": "search", "2": "music", "3": "timer" };

/** What can live in the folded notch. With two at once, one takes the bubble. */
type Activity = "music" | "timer";

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
  const timerSettings = settings?.settings.timer;
  const timerEnabled = timerSettings?.enabled ?? false;
  useApplyAppearance(appearance, settings?.systemAccentColor ?? null);

  const media = useNowPlaying(musicEnabled);
  const timer = useTimer(timerEnabled);
  /** The activity the user turned to last: it gets the notch, the other the bubble. */
  const [primary, setPrimary] = useState<Activity>("music");
  const [timerAlert, setTimerAlert] = useState<TimerFinished | null>(null);
  const glow = useArtworkGlow(media?.artwork ?? null, Boolean(music?.artworkTint));
  const wallpaper = useWallpaper(appearance?.material === "mica", session.id);
  const backdrop = resolveBackdrop(appearance?.material, wallpaper, snapshot);
  const tabs = useMemo(() => {
    const enabled: Record<WidgetTab, boolean> = { search: searchEnabled, music: musicEnabled, timer: timerEnabled };
    return TAB_ORDER.filter((candidate) => enabled[candidate]);
  }, [searchEnabled, musicEnabled, timerEnabled]);

  // Which activity takes the notch, which one the bubble beside it.
  const timerActive = isTimerActive(timer);
  const musicShown = Boolean(media && music?.showInNotch);
  const timerShown = Boolean(timerSettings?.showInNotch && (timerActive || timerAlert));
  const activities: Activity[] = [];
  if (musicShown) activities.push("music");
  if (timerShown) activities.push("timer");
  const mainActivity: Activity | null =
    timerAlert && timerShown ? "timer" : activities.includes(primary) ? primary : (activities[0] ?? null);
  const bubbleActivity: Activity | null = activities.find((activity) => activity !== mainActivity) ?? null;
  const activeTab = tabs.includes(tab) ? tab : (tabs[0] ?? "search");

  const modeRef = useRef<NotchMode>("idle");
  const sessionRef = useRef(session);
  const mediaRef = useRef(media);
  const timerRef = useRef(timer);
  const timerAlertRef = useRef(timerAlert);
  const timerSettingsRef = useRef(timerSettings);
  const mainActivityRef = useRef(mainActivity);
  const bubbleActivityRef = useRef(bubbleActivity);
  const tabsRef = useRef(tabs);
  const tabRef = useRef<WidgetTab>(activeTab);
  const openedAtRef = useRef(0);
  const engagedRef = useRef(false);
  const leaveTimerRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    modeRef.current = mode;
    sessionRef.current = session;
    mediaRef.current = media;
    timerRef.current = timer;
    timerAlertRef.current = timerAlert;
    timerSettingsRef.current = timerSettings;
    mainActivityRef.current = mainActivity;
    bubbleActivityRef.current = bubbleActivity;
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
    ({ source, zone, backdrop: screenBehind }: NotchOpenedPayload) => {
      engagedRef.current = false;
      openedAtRef.current = performance.now();
      setSnapshot(screenBehind ?? null);
      setSession((previous) => ({ id: previous.id + 1, source }));
      // Hovering an activity (the notch or its bubble) means "show me that";
      // the keyboard shortcut means "search".
      const availableTabs = tabsRef.current;
      const pointed = source === "hover" ? (zone === "bubble" ? bubbleActivityRef.current : mainActivityRef.current) : null;
      const fallback = availableTabs.includes("search") ? "search" : (availableTabs[0] ?? "search");
      setTab(pointed && availableTabs.includes(pointed) ? pointed : fallback);
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
  // A period ended: the notch peeks with the news (the timer takes the notch).
  useBackendEvent<TimerFinished>(BACKEND_EVENTS.timerFinished, (finished) => {
    if (!timerSettingsRef.current?.showInNotch) return;
    setTimerAlert(finished);
    setPrimary("timer");
    setMode((current) => (current === "open" ? current : "peek"));
    window.clearTimeout(peekTimerRef.current);
    peekTimerRef.current = window.setTimeout(() => {
      setTimerAlert(null);
      setMode((current) => (current === "peek" ? "idle" : current));
    }, TIMER_ALERT_MS);
  });
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
    setPrimary("music");
    // The timer lives in a ref, not in this effect's cleanup: pausing during
    // the peek must not cancel the fold back.
    window.clearTimeout(peekTimerRef.current);
    peekTimerRef.current = window.setTimeout(
      () => setMode((current) => (current === "peek" ? "idle" : current)),
      PEEK_DURATION_MS,
    );
  }, [currentTrack, isPlaying, music?.peekOnTrackChange, music?.showInNotch]);

  useEffect(() => () => window.clearTimeout(peekTimerRef.current), []);

  // Starting a timer brings it to the notch; music moves to the bubble.
  const wasTimerActiveRef = useRef(false);
  useEffect(() => {
    if (timerActive && !wasTimerActiveRef.current) setPrimary("timer");
    wasTimerActiveRef.current = timerActive;
  }, [timerActive]);

  // Closing the panel on Music or Timer makes that activity the main one:
  // the user looked at it last.
  const previousModeRef = useRef(mode);
  useEffect(() => {
    const last = tabRef.current;
    if (previousModeRef.current === "open" && mode !== "open" && (last === "music" || last === "timer")) {
      setPrimary(last);
    }
    previousModeRef.current = mode;
  }, [mode]);

  // Rust needs the bubble's hover zone only while it is shown.
  const hasBubble = bubbleActivity !== null;
  useEffect(() => {
    backend.notchSetLayout(hasBubble).catch((error: unknown) => console.error("Cannot update the notch layout", error));
  }, [hasBubble]);

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

  const readTimerProgress = useCallback(() => {
    const current = timerRef.current;
    // An ended countdown is no longer active: its light stays full during the alert.
    if (!isTimerActive(current)) return timerAlertRef.current ? 1 : null;
    return elapsedRatio(current);
  }, []);

  const liveMode = mode === "open" || mode === "accent" || accent?.visible ? "open" : mode;
  const timerGlow = timerColor(timerAlert && !timerActive ? timerAlert.phase : (timer?.phase ?? null));
  const musicLive: NotchLiveActivity | null =
    media && music?.showInNotch
      ? {
          content: <NowPlayingLive media={media} mode={liveMode} showAudioBars={Boolean(music.showAudioBars)} />,
          playing: media.playing,
          glow,
          readProgress,
        }
      : null;
  const timerLive: NotchLiveActivity | null = timerShown
    ? {
        content: <TimerLive timer={timerActive ? timer : null} alert={timerAlert} mode={liveMode} />,
        playing: timer?.status === "running" || Boolean(timerAlert),
        glow: timerGlow,
        readProgress: readTimerProgress,
      }
    : null;
  const live = mainActivity === "music" ? musicLive : mainActivity === "timer" ? timerLive : null;

  let bubble: NotchBubble | null = null;
  if (bubbleActivity === "music" && media) {
    bubble = { id: "music", content: <MusicBubbleContent media={media} />, color: glow, playing: media.playing, readProgress };
  } else if (bubbleActivity === "timer" && timerActive) {
    bubble = {
      id: "timer",
      content: <TimerBubbleContent timer={timer} />,
      color: timerGlow,
      playing: timer.status === "running",
      readProgress: readTimerProgress,
    };
  }

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
      bubble={bubble}
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
          timerRunning={timer?.status === "running"}
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
          {timerEnabled && timerSettings && (
            <div className="widget-view" hidden={activeTab !== "timer"}>
              <TimerPanel
                timer={timer}
                settings={timerSettings}
                isActive={mode === "open" && activeTab === "timer"}
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
