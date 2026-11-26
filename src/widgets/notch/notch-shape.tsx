import { useEffect, useRef, type CSSProperties, type PointerEvent, type ReactNode } from "react";

import "./notch-shape.css";

/** Progress light refresh; the CSS transition fills the gaps linearly. */
const PROGRESS_TICK_MS = 500;

/**
 * - idle: the small black notch (wider while media plays)
 * - peek: briefly expanded to show the new track, still non-interactive
 * - open: the full widget panel, focused
 * - accent: the Quick Accent bar, non-interactive (focus stays in the app typed in)
 */
export type NotchMode = "idle" | "peek" | "open" | "accent";

/**
 * Image behind the glass materials:
 * - "screen": the wallpaper, laid out over the whole monitor (Mica)
 * - "window": a snapshot of exactly what the notch window covers (Acrylic)
 */
export interface NotchBackdrop {
  image: string;
  fit: "screen" | "window";
}

/** Media playing: the notch edge turns into a progress light. */
export interface NotchLiveActivity {
  /** Shown when the notch peeks (new track). */
  content: ReactNode;
  playing: boolean;
  /** Glow color taken from the cover; the accent is used when null. */
  glow: string | null;
  /** Playback progress from 0 to 1, or null when unknown (live stream). */
  readProgress: () => number | null;
}

/** A second activity, shown in a small bubble beside the folded notch. */
export interface NotchBubble {
  /** Changes when another activity takes the bubble. */
  id: string;
  content: ReactNode;
  /** Ring color; the accent when null. */
  color: string | null;
  playing: boolean;
  /** From 0 to 1, or null when unknown. */
  readProgress: () => number | null;
}

const BUBBLE_SIZE = 30;
const BUBBLE_STROKE = 2;
const BUBBLE_RADIUS = (BUBBLE_SIZE - BUBBLE_STROKE) / 2;
const BUBBLE_CIRCUMFERENCE = 2 * Math.PI * BUBBLE_RADIUS;

/**
 * Writes the playback progress into `--media-progress` twice a second. The
 * property is registered and transitions linearly, so the light glides
 * continuously without re-rendering anything.
 */
function useProgressLight(canvasRef: React.RefObject<HTMLDivElement | null>, live: NotchLiveActivity | null): void {
  const playing = Boolean(live?.playing);
  const readProgress = live?.readProgress;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (!readProgress) {
      canvas.style.removeProperty("--media-progress");
      delete canvas.dataset.progress;
      return;
    }
    const update = () => {
      const progress = readProgress();
      canvas.dataset.progress = progress === null ? "unknown" : "known";
      canvas.style.setProperty("--media-progress", String(progress ?? 1));
    };
    update();
    if (!playing) return;
    const timer = window.setInterval(update, PROGRESS_TICK_MS);
    return () => window.clearInterval(timer);
  }, [canvasRef, playing, readProgress]);
}

/** The bubble's ring, written directly (no re-render) twice a second. */
function useBubbleRing(ringRef: React.RefObject<SVGCircleElement | null>, bubble: NotchBubble | null): void {
  const readProgress = bubble?.readProgress;
  const playing = Boolean(bubble?.playing);
  useEffect(() => {
    const ring = ringRef.current;
    if (!ring || !readProgress) return;
    const update = () => {
      const progress = readProgress() ?? 1;
      ring.style.strokeDashoffset = String(BUBBLE_CIRCUMFERENCE * (1 - progress));
    };
    update();
    if (!playing) return;
    const timer = window.setInterval(update, PROGRESS_TICK_MS);
    return () => window.clearInterval(timer);
  }, [ringRef, readProgress, playing]);
}

function Bubble({ bubble, visible }: { bubble: NotchBubble | null; visible: boolean }) {
  const ringRef = useRef<SVGCircleElement>(null);
  useBubbleRing(ringRef, bubble);
  const style = bubble?.color ? ({ "--bubble-color": bubble.color } as CSSProperties) : undefined;
  return (
    <div className="notch-bubble" data-visible={visible && bubble !== null} data-playing={Boolean(bubble?.playing)} style={style} aria-hidden="true">
      <svg className="notch-bubble__ring" width={BUBBLE_SIZE} height={BUBBLE_SIZE} viewBox={`0 0 ${BUBBLE_SIZE} ${BUBBLE_SIZE}`}>
        <circle className="notch-bubble__track" cx={BUBBLE_SIZE / 2} cy={BUBBLE_SIZE / 2} r={BUBBLE_RADIUS} />
        <circle
          ref={ringRef}
          className="notch-bubble__fill"
          cx={BUBBLE_SIZE / 2}
          cy={BUBBLE_SIZE / 2}
          r={BUBBLE_RADIUS}
          strokeDasharray={BUBBLE_CIRCUMFERENCE}
        />
      </svg>
      <span key={bubble?.id} className="notch-bubble__content">
        {bubble?.content}
      </span>
    </div>
  );
}

interface NotchShapeProps {
  mode: NotchMode;
  /** Height of the open panel content, in CSS pixels. */
  openHeight: number;
  backdrop: NotchBackdrop | null;
  live: NotchLiveActivity | null;
  /** A second activity beside the folded notch. */
  bubble: NotchBubble | null;
  /** Drawn over the idle notch (Quick Accent), with the width it needs. */
  overlay: { content: ReactNode; width: number } | null;
  children: ReactNode;
  /** Pointer pressed on the transparent area around the notch. */
  onBackdropPointerDown: () => void;
  onPanelPointerEnter: () => void;
  onPanelPointerLeave: () => void;
}

/** The window is centred at the top of its monitor, so its offset is known. */
function screenFitStyle(): CSSProperties {
  return {
    left: -(window.screen.width - window.innerWidth) / 2,
    top: 0,
    width: window.screen.width,
    height: window.screen.height,
  };
}

/**
 * The morphing notch. Widgets render inside it; it owns only the shape, its
 * material, its light and the open/close choreography.
 */
export function NotchShape({
  mode,
  openHeight,
  backdrop,
  live,
  bubble,
  overlay,
  children,
  onBackdropPointerDown,
  onPanelPointerEnter,
  onPanelPointerLeave,
}: NotchShapeProps) {
  const canvasRef = useRef<HTMLDivElement>(null);
  useProgressLight(canvasRef, live);
  // Kept after it ends so the bubble can fold back into the notch.
  const lastBubbleRef = useRef<NotchBubble | null>(null);
  if (bubble) lastBubbleRef.current = bubble;
  const style = {
    "--notch-open-h": `${openHeight}px`,
    ...(live?.glow ? { "--media-glow": live.glow } : {}),
    ...(overlay ? { "--notch-accent-w": `${overlay.width}px` } : {}),
  } as CSSProperties;

  const handleCanvasPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    // The clipped-out area of the body does not receive pointer events, so a
    // press that reaches the canvas itself happened outside the notch.
    if (event.target === event.currentTarget) onBackdropPointerDown();
  };

  return (
    <div
      ref={canvasRef}
      className="notch-canvas"
      data-mode={mode}
      data-live={Boolean(live)}
      data-playing={Boolean(live?.playing)}
      style={style}
      onPointerDown={handleCanvasPointerDown}
    >
      <div className="notch-spill" aria-hidden="true" />
      <div className="notch-shadow" aria-hidden="true" />
      <div className="notch-shoulder notch-shoulder--left" aria-hidden="true" />
      <div className="notch-shoulder notch-shoulder--right" aria-hidden="true" />
      <div className="notch-rim" aria-hidden="true" />
      <Bubble bubble={bubble ?? lastBubbleRef.current} visible={bubble !== null && (mode === "idle" || mode === "peek")} />
      <div className="notch-live-ring" aria-hidden="true">
        <span className="notch-live-ring__track" />
        <span className="notch-live-ring__fill" />
        <span className="notch-live-ring__comet" />
      </div>
      <div className="notch-body" onPointerEnter={onPanelPointerEnter} onPointerLeave={onPanelPointerLeave}>
        {backdrop && (
          <div
            className="notch-backdrop"
            data-fit={backdrop.fit}
            aria-hidden="true"
            style={{
              backgroundImage: `url("${backdrop.image}")`,
              ...(backdrop.fit === "screen" ? screenFitStyle() : {}),
            }}
          />
        )}
        <div className="notch-surface" aria-hidden="true" />
        <div className="notch-idle-cover" aria-hidden="true" />
        {live?.content}
        {overlay?.content}
        <div className="notch-content" inert={mode !== "open"}>
          {children}
        </div>
      </div>
    </div>
  );
}
