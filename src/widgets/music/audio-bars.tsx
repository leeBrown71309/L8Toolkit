import { useEffect, useRef } from "react";

import { backend, BACKEND_EVENTS } from "../../shared/backend";
import { useBackendEvent } from "../../shared/use-backend-event";
import { writeSimulatedLevels } from "./simulated-levels";

import "./audio-bars.css";

/** Bands sent by Rust (audio/mod.rs BAND_COUNT); mirrored around the centre. */
const BAND_COUNT = 15;
/** A silent bar is still a dot, so the row reads as "ready" not "broken". */
const MIN_SCALE = 0.14;
/** Share of the gap to the new level covered per 60 Hz frame. */
const RISE_PER_FRAME = 0.45;
const FALL_PER_FRAME = 0.16;
const FRAME_MS = 1000 / 60;
const REST_EPSILON = 0.002;
/**
 * Playing, yet nothing heard on this PC for this long: the sound is elsewhere
 * (a phone bridged through NectarLink or Phone Link), so the bars switch to a
 * stand-in motion. Real pauses in music rarely last this long in pure silence.
 */
const SILENCE_BEFORE_SIMULATION_MS = 1500;
/** The stand-in motion grows in instead of appearing at full height. */
const SIMULATION_FADE_IN_MS = 900;
const SIMULATION_TICK_MS = 33;

/**
 * Band order across the row: highs on the edges, bass in the middle, the
 * same on both sides, so the shape stays centred like the notch itself.
 */
const BAR_BANDS = [
  ...Array.from({ length: BAND_COUNT }, (_, index) => BAND_COUNT - 1 - index),
  ...Array.from({ length: BAND_COUNT }, (_, index) => index),
];

interface AudioBarsProps {
  /** The bars are on screen and something plays: capture the sound. */
  active: boolean;
}

/**
 * The sound being played, as bars across the folded notch. Rust sends band
 * levels ~30 times a second; each display frame eases the bars toward them
 * and writes transforms directly, so React never re-renders per frame.
 * When the media plays somewhere this PC cannot hear, a stand-in motion
 * takes over (see simulated-levels.ts).
 */
export function AudioBars({ active }: AudioBarsProps) {
  const barRefs = useRef<(HTMLSpanElement | null)[]>([]);
  const targets = useRef(new Float32Array(BAND_COUNT));
  const levels = useRef(new Float32Array(BAND_COUNT));
  const frame = useRef(0);
  const lastPaint = useRef(0);
  const rowRef = useRef<HTMLSpanElement>(null);
  /** When real sound was last received (performance.now()). */
  const lastHeardAt = useRef(0);
  /** When the stand-in motion started, null while real sound drives the bars. */
  const simulationStartedAt = useRef<number | null>(null);
  const simulated = useRef(new Float32Array(BAND_COUNT));

  const paint = (now: number) => {
    const elapsed = lastPaint.current ? Math.min(100, now - lastPaint.current) : FRAME_MS;
    lastPaint.current = now;
    const frames = elapsed / FRAME_MS;
    let moving = false;
    for (let band = 0; band < BAND_COUNT; band += 1) {
      const target = targets.current[band] ?? 0;
      const current = levels.current[band] ?? 0;
      const perFrame = target > current ? RISE_PER_FRAME : FALL_PER_FRAME;
      const next = current + (target - current) * (1 - (1 - perFrame) ** frames);
      levels.current[band] = Math.abs(target - next) < REST_EPSILON ? target : next;
      if (levels.current[band] !== target) moving = true;
    }
    barRefs.current.forEach((bar, index) => {
      if (!bar) return;
      const level = levels.current[BAR_BANDS[index] ?? 0] ?? 0;
      bar.style.transform = `scaleY(${MIN_SCALE + (1 - MIN_SCALE) * level})`;
    });
    // Keep animating while levels move; Rust keeps them moving while it plays.
    frame.current = moving ? requestAnimationFrame(paint) : 0;
    if (!moving) lastPaint.current = 0;
  };

  const schedulePaint = () => {
    if (frame.current === 0) frame.current = requestAnimationFrame(paint);
  };

  const setSimulating = (simulating: boolean) => {
    simulationStartedAt.current = simulating ? performance.now() : null;
    if (rowRef.current) rowRef.current.dataset.source = simulating ? "simulated" : "audio";
  };

  useBackendEvent<number[]>(BACKEND_EVENTS.audioLevels, (payload) => {
    const heard = payload.some((level) => level > 0);
    if (heard) {
      lastHeardAt.current = performance.now();
      if (simulationStartedAt.current !== null) setSimulating(false);
    } else if (simulationStartedAt.current !== null) {
      return; // Silence must not flatten the stand-in motion.
    }
    for (let band = 0; band < BAND_COUNT; band += 1) targets.current[band] = (payload[band] ?? 0) / 255;
    schedulePaint();
  });

  useEffect(() => {
    backend.audioBarsSetVisible(active).catch((error: unknown) => console.error("Cannot toggle the audio bars", error));
    if (!active) {
      setSimulating(false);
      targets.current.fill(0);
      schedulePaint();
      return;
    }
    // Silence is judged from the moment playback shows, not from the last song.
    lastHeardAt.current = performance.now();
    const timer = window.setInterval(() => {
      const now = performance.now();
      if (simulationStartedAt.current === null) {
        if (now - lastHeardAt.current < SILENCE_BEFORE_SIMULATION_MS) return;
        setSimulating(true);
      }
      const startedAt = simulationStartedAt.current ?? now;
      const fadeIn = Math.min(1, (now - startedAt) / SIMULATION_FADE_IN_MS);
      writeSimulatedLevels(now / 1000, simulated.current);
      for (let band = 0; band < BAND_COUNT; band += 1) targets.current[band] = (simulated.current[band] ?? 0) * fadeIn;
      schedulePaint();
    }, SIMULATION_TICK_MS);
    return () => window.clearInterval(timer);
    // The helpers only touch refs, so `active` is the only real input.
  }, [active]);

  useEffect(
    () => () => {
      cancelAnimationFrame(frame.current);
      frame.current = 0;
      backend.audioBarsSetVisible(false).catch((error: unknown) => console.error("Cannot stop the audio bars", error));
    },
    [],
  );

  return (
    <span ref={rowRef} className="audio-bars" data-active={active} data-source="audio" aria-hidden="true">
      {BAR_BANDS.map((band, index) => (
        <span
          // Positions are fixed for the component's lifetime.
          key={index}
          ref={(element) => {
            barRefs.current[index] = element;
          }}
          className="audio-bars__bar"
          data-band={band}
        />
      ))}
    </span>
  );
}
