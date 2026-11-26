/**
 * Geometry of the wavy progress line: the played part undulates, the rest
 * stays flat. Pure functions, so the drawing stays out of React.
 */

/** Horizontal distance between two crests, in CSS pixels. */
export const WAVE_LENGTH = 22;
/** Height of a crest above the middle line while playing. */
export const WAVE_AMPLITUDE = 3;
/** Gap left on each side of the thumb. */
export const THUMB_GAP = 5;
/** Horizontal resolution of the curve: one point every 2px is smooth at any DPI. */
const SAMPLE_STEP = 2;

export interface WaveLayout {
  /** SVG path of the played part (a sine wave, flat when the amplitude is 0). */
  played: string;
  /** SVG path of the part still to play (a straight line), or "" when none. */
  remaining: string;
  /** Thumb position, in pixels from the left. */
  thumbX: number;
}

/**
 * Lays the line out for a track `width` pixels wide and `height` tall.
 * `phase` (radians) slides the wave along; `amplitude` fades it in and out.
 * The wave eases to zero over its last half-wavelength, so it meets the
 * thumb on the middle line instead of being cut mid-crest.
 */
export function layoutWave(width: number, height: number, fraction: number, amplitude: number, phase: number): WaveLayout {
  const middle = height / 2;
  const thumbX = Math.min(width, Math.max(0, fraction * width));
  const playedEnd = Math.max(0, thumbX - THUMB_GAP);
  const taper = WAVE_LENGTH / 2;

  let played = "";
  if (playedEnd > 0.5) {
    const points: string[] = [];
    for (let x = 0; x <= playedEnd; x += SAMPLE_STEP) points.push(point(x, playedEnd, middle, amplitude, phase, taper));
    if (points.length === 0 || (playedEnd % SAMPLE_STEP) !== 0) {
      points.push(point(playedEnd, playedEnd, middle, amplitude, phase, taper));
    }
    played = `M${points.join("L")}`;
  }

  const remainingStart = Math.min(width, thumbX + THUMB_GAP);
  const remaining = remainingStart < width - 0.5 ? `M${round(remainingStart)} ${round(middle)}H${round(width)}` : "";
  return { played, remaining, thumbX };
}

function point(x: number, end: number, middle: number, amplitude: number, phase: number, taper: number): string {
  const distanceToEnd = end - x;
  const envelope = taper > 0 ? Math.min(1, distanceToEnd / taper) : 1;
  const y = middle + amplitude * envelope * Math.sin((x / WAVE_LENGTH) * Math.PI * 2 - phase);
  return `${round(x)} ${round(y)}`;
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
