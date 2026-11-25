import { useEffect, useRef, type CSSProperties, type PointerEvent, type ReactNode } from "react";

import "./notch-shape.css";

/** Progress light refresh; the CSS transition fills the gaps linearly. */
const PROGRESS_TICK_MS = 500;

/**
 * - idle: the small black notch (wider while media plays)
 * - peek: briefly expanded to show the new track, still non-interactive
 * - open: the full widget panel, focused
 */
export type NotchMode = "idle" | "peek" | "open";

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

interface NotchShapeProps {
  mode: NotchMode;
  /** Height of the open panel content, in CSS pixels. */
  openHeight: number;
  backdrop: NotchBackdrop | null;
  live: NotchLiveActivity | null;
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
  children,
  onBackdropPointerDown,
  onPanelPointerEnter,
  onPanelPointerLeave,
}: NotchShapeProps) {
  const canvasRef = useRef<HTMLDivElement>(null);
  useProgressLight(canvasRef, live);
  const style = {
    "--notch-open-h": `${openHeight}px`,
    ...(live?.glow ? { "--media-glow": live.glow } : {}),
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
        <div className="notch-content" inert={mode !== "open"}>
          {children}
        </div>
      </div>
    </div>
  );
}
