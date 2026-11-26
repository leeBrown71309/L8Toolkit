import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

import { useI18n } from "../../i18n/i18n-context";
import type { NowPlaying } from "../../shared/types";
import { currentPositionMs, formatDuration, usePlaybackClock } from "./use-now-playing";
import { layoutWave, WAVE_AMPLITUDE } from "./wave-path";

/** Keep showing the requested position until the player confirms the seek. */
const SEEK_CONFIRM_TIMEOUT_MS = 1600;
const KEYBOARD_SEEK_STEP_MS = 10_000;
/** Height of the drawing, in CSS pixels: room for the crests and the stroke. */
const WAVE_HEIGHT = 14;
/** The wave travels one wavelength in this time while playing. */
const WAVE_PERIOD_MS = 1500;
/** Share of the gap to the target amplitude covered per 60 Hz frame. */
const AMPLITUDE_EASE_PER_FRAME = 0.12;
const FRAME_MS = 1000 / 60;

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

const prefersReducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Seekable progress line. The played part is a travelling wave (it settles
 * into a straight line on pause), the rest a flat line, both in the cover's
 * color. A requestAnimationFrame loop rewrites the SVG paths directly, so the
 * wave moves at the display's refresh rate without any React render.
 */
export function ProgressBar({ media, onSeek, children }: ProgressBarProps) {
  const { t } = useI18n();
  const trackRef = useRef<HTMLDivElement>(null);
  const playedRef = useRef<SVGPathElement>(null);
  const remainingRef = useRef<SVGPathElement>(null);
  const thumbRef = useRef<HTMLSpanElement>(null);
  const widthRef = useRef(0);
  const amplitudeRef = useRef(0);
  const phaseRef = useRef(0);
  /** Draws one frame; set by the animation effect, called again on resize. */
  const repaintRef = useRef<(() => void) | null>(null);
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

  // The path is laid out in pixels, so it follows the track's real width.
  useEffect(() => {
    const track = trackRef.current;
    if (!track) return;
    // The panel may mount on a hidden tab (width 0): draw again once it shows.
    const observer = new ResizeObserver(([entry]) => {
      widthRef.current = entry?.contentRect.width ?? track.clientWidth;
      repaintRef.current?.();
    });
    observer.observe(track);
    widthRef.current = track.clientWidth;
    return () => observer.disconnect();
  }, [duration > 0]);

  useEffect(() => {
    if (duration <= 0) return;
    const reducedMotion = prefersReducedMotion();
    const targetAmplitude = media.playing ? WAVE_AMPLITUDE : 0;
    let frame = 0;
    let lastTime = performance.now();

    const paint = (now: number) => {
      // Capped, so a frame after a long pause (hidden window) does not jump.
      const elapsed = Math.min(100, now - lastTime);
      const frames = elapsed / FRAME_MS;
      lastTime = now;
      const amplitude = amplitudeRef.current;
      const settled = Math.abs(targetAmplitude - amplitude) < 0.02;
      amplitudeRef.current = settled
        ? targetAmplitude
        : amplitude + (targetAmplitude - amplitude) * (1 - (1 - AMPLITUDE_EASE_PER_FRAME) ** frames);
      if (media.playing && !reducedMotion) {
        phaseRef.current = (phaseRef.current + (elapsed / WAVE_PERIOD_MS) * Math.PI * 2) % (Math.PI * 2);
      }

      const position =
        dragFraction !== null ? dragFraction * duration : pendingSeek ? pendingSeek.positionMs : currentPositionMs(media);
      const fraction = Math.min(1, Math.max(0, position / duration));
      const layout = layoutWave(widthRef.current, WAVE_HEIGHT, fraction, amplitudeRef.current, phaseRef.current);
      playedRef.current?.setAttribute("d", layout.played);
      remainingRef.current?.setAttribute("d", layout.remaining);
      if (thumbRef.current) thumbRef.current.style.transform = `translateX(${layout.thumbX}px)`;

      const animating = (media.playing && !reducedMotion) || amplitudeRef.current !== targetAmplitude;
      if (animating) frame = requestAnimationFrame(paint);
    };
    repaintRef.current = () => {
      cancelAnimationFrame(frame);
      paint(performance.now());
    };
    // First drawing right away: animation frames pause while the window is hidden.
    paint(lastTime);
    return () => {
      repaintRef.current = null;
      cancelAnimationFrame(frame);
    };
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
        <svg className="progress__wave" height={WAVE_HEIGHT} aria-hidden="true">
          <path ref={remainingRef} className="progress__remaining" />
          <path ref={playedRef} className="progress__played" />
        </svg>
        <span ref={thumbRef} className="progress__thumb" />
      </div>
      <div className="progress__row">
        <span className="progress__time">{formatDuration(shownPosition)}</span>
        {children}
        <span className="progress__time progress__time--end">{formatDuration(duration)}</span>
      </div>
    </div>
  );
}
