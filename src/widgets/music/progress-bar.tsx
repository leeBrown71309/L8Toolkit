import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

import { useI18n } from "../../i18n/i18n-context";
import type { NowPlaying } from "../../shared/types";
import { currentPositionMs, formatDuration, usePlaybackClock } from "./use-now-playing";

/** Keep showing the requested position until the player confirms the seek. */
const SEEK_CONFIRM_TIMEOUT_MS = 1600;
const KEYBOARD_SEEK_STEP_MS = 10_000;

interface PendingSeek {
  positionMs: number;
  requestedAt: number;
}

interface ProgressBarProps {
  media: NowPlaying;
  onSeek: (positionMs: number) => void;
  /** Rendered between the elapsed time and the duration (the controls). */
  children?: ReactNode;
}

/**
 * Seekable progress bar. The fill is driven by requestAnimationFrame writing
 * transforms directly (no React render per frame), so it glides at the
 * display's refresh rate without costing the rest of the UI anything.
 */
export function ProgressBar({ media, onSeek, children }: ProgressBarProps) {
  const { t } = useI18n();
  const trackRef = useRef<HTMLDivElement>(null);
  const fillRef = useRef<HTMLDivElement>(null);
  const headRef = useRef<HTMLDivElement>(null);
  const [dragFraction, setDragFraction] = useState<number | null>(null);
  const [pendingSeek, setPendingSeek] = useState<PendingSeek | null>(null);
  const clockPosition = usePlaybackClock(media);
  const duration = media.durationMs;

  // A newer reading from the player supersedes the optimistic position.
  useEffect(() => {
    if (pendingSeek && media.updatedAtMs > pendingSeek.requestedAt) setPendingSeek(null);
  }, [media.updatedAtMs, pendingSeek]);

  useEffect(() => {
    if (!pendingSeek) return;
    const timer = window.setTimeout(() => setPendingSeek(null), SEEK_CONFIRM_TIMEOUT_MS);
    return () => window.clearTimeout(timer);
  }, [pendingSeek]);

  useEffect(() => {
    if (duration <= 0) return;
    let frame = 0;
    const paint = () => {
      const position =
        dragFraction !== null
          ? dragFraction * duration
          : pendingSeek
            ? pendingSeek.positionMs
            : currentPositionMs(media);
      const fraction = Math.min(1, Math.max(0, position / duration));
      if (fillRef.current) fillRef.current.style.transform = `scaleX(${fraction})`;
      // The head is a zero-width anchor: moving it never overflows the track.
      if (headRef.current) headRef.current.style.left = `${fraction * 100}%`;
      if (media.playing && dragFraction === null) frame = requestAnimationFrame(paint);
    };
    paint();
    return () => cancelAnimationFrame(frame);
  }, [media, duration, dragFraction, pendingSeek]);

  // Live streams have no duration: only the controls make sense.
  if (duration <= 0) return <div className="progress__row progress__row--controls-only">{children}</div>;

  const fractionAt = (clientX: number) => {
    const rect = trackRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return 0;
    return Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
  };

  const commitSeek = (positionMs: number) => {
    // Whole milliseconds: a click lands between two of them.
    const clamped = Math.round(Math.min(duration, Math.max(0, positionMs)));
    setPendingSeek({ positionMs: clamped, requestedAt: Date.now() });
    onSeek(clamped);
  };

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!media.canSeek) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragFraction(fractionAt(event.clientX));
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (dragFraction !== null) setDragFraction(fractionAt(event.clientX));
  };

  const handlePointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (dragFraction === null) return;
    commitSeek(fractionAt(event.clientX) * duration);
    setDragFraction(null);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!media.canSeek) return;
    const step = event.key === "ArrowRight" ? KEYBOARD_SEEK_STEP_MS : event.key === "ArrowLeft" ? -KEYBOARD_SEEK_STEP_MS : 0;
    if (step === 0) return;
    event.preventDefault();
    event.stopPropagation();
    commitSeek(currentPositionMs(media) + step);
  };

  const shownPosition =
    dragFraction !== null ? dragFraction * duration : pendingSeek ? pendingSeek.positionMs : clockPosition;

  return (
    <div className="progress" data-seekable={media.canSeek} data-dragging={dragFraction !== null}>
      <div
        ref={trackRef}
        className="progress__hit-area"
        role="slider"
        tabIndex={media.canSeek ? 0 : -1}
        aria-label={t("music.position")}
        aria-valuemin={0}
        aria-valuemax={Math.round(duration / 1000)}
        aria-valuenow={Math.round(shownPosition / 1000)}
        aria-valuetext={t("music.positionValue", { position: formatDuration(shownPosition), duration: formatDuration(duration) })}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={() => setDragFraction(null)}
        onKeyDown={handleKeyDown}
      >
        <div className="progress__track">
          <div ref={fillRef} className="progress__fill" />
        </div>
        <div ref={headRef} className="progress__head">
          <span className="progress__thumb" />
        </div>
      </div>
      <div className="progress__row">
        <span className="progress__time">{formatDuration(shownPosition)}</span>
        {children}
        <span className="progress__time progress__time--end">{formatDuration(duration)}</span>
      </div>
    </div>
  );
}
