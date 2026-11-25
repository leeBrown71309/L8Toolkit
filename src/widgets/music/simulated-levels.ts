/**
 * Stand-in levels for media whose sound never reaches this PC (music played
 * on a phone and only controlled from here through NectarLink, Phone Link
 * and the like): Windows knows it plays, but there is nothing to measure.
 *
 * The motion is calmer and more regular than real audio on purpose: it says
 * "playing" without pretending to follow the music.
 */

/** Roughly 112 BPM: a soft pulse, mostly felt in the bass bands. */
const PULSE_HZ = 1.87;
const BASE_LEVEL = 0.12;
const WAVE_LEVEL = 0.46;
const PULSE_LEVEL = 0.28;

/** Fills `output` with levels in [0, 1] for time `seconds`, band 0 = bass. */
export function writeSimulatedLevels(seconds: number, output: Float32Array): void {
  const bandCount = output.length;
  const pulse = Math.max(0, Math.sin(seconds * Math.PI * 2 * PULSE_HZ)) ** 4;
  for (let band = 0; band < bandCount; band += 1) {
    const position = bandCount > 1 ? band / (bandCount - 1) : 0;
    // Two slow waves per band, with incommensurate speeds so bars never sync.
    const wave =
      0.6 * (0.5 + 0.5 * Math.sin(seconds * (1.3 + band * 0.37) + band * 2.1)) +
      0.4 * (0.5 + 0.5 * Math.sin(seconds * (2.9 + band * 0.23) + band * 0.7));
    const pulseShare = PULSE_LEVEL * pulse * (1 - position * 0.7);
    // Highs sit lower, as in most music.
    const tilt = 1 - position * 0.45;
    output[band] = Math.min(1, (BASE_LEVEL + WAVE_LEVEL * wave + pulseShare) * tilt);
  }
}
