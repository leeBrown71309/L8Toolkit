import { useEffect, useState } from "react";

import { backend, BACKEND_EVENTS } from "../../shared/backend";
import type { NowPlaying } from "../../shared/types";
import { useBackendEvent } from "../../shared/use-backend-event";

/** Current media session, kept in sync with the Rust media watcher. */
export function useNowPlaying(enabled: boolean): NowPlaying | null {
  const [media, setMedia] = useState<NowPlaying | null>(null);

  useEffect(() => {
    if (!enabled) {
      setMedia(null);
      return;
    }
    let alive = true;
    backend
      .mediaState()
      .then((state) => {
        if (alive) setMedia(state);
      })
      .catch((error: unknown) => console.error("Cannot read the media state", error));
    return () => {
      alive = false;
    };
  }, [enabled]);

  useBackendEvent<NowPlaying | null>(BACKEND_EVENTS.mediaChanged, (next) => {
    if (enabled) setMedia(next);
  });

  return enabled ? media : null;
}

/**
 * Position "now": Rust only reports changes, so while playing the position
 * is extrapolated from the last reading instead of polled.
 */
export function currentPositionMs(media: NowPlaying, now: number = Date.now()): number {
  if (!media.playing) return media.positionMs;
  const position = media.positionMs + Math.max(0, now - media.updatedAtMs);
  return media.durationMs > 0 ? Math.min(position, media.durationMs) : position;
}

/** Re-renders twice a second while playing, for the time labels. */
export function usePlaybackClock(media: NowPlaying | null): number {
  const [, setTick] = useState(0);
  const playing = Boolean(media?.playing);

  useEffect(() => {
    if (!playing) return;
    const timer = window.setInterval(() => setTick((tick) => tick + 1), 500);
    return () => window.clearInterval(timer);
  }, [playing]);

  return media ? currentPositionMs(media) : 0;
}

export function formatDuration(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}
