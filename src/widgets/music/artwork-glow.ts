import { useEffect, useState } from "react";

/** Sampling a 24px thumbnail is plenty to find a cover's dominant hue. */
const SAMPLE_SIZE = 24;
const MAX_CACHED_COLORS = 24;
const MIN_SATURATION = 0.45;
const MAX_SATURATION = 0.85;
const MIN_LIGHTNESS = 0.56;
const MAX_LIGHTNESS = 0.68;

const glowCache = new Map<string, string>();

interface Hsl {
  hue: number;
  saturation: number;
  lightness: number;
}

function rgbToHsl(red: number, green: number, blue: number): Hsl {
  const r = red / 255;
  const g = green / 255;
  const b = blue / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const lightness = (max + min) / 2;
  const delta = max - min;
  if (delta === 0) return { hue: 0, saturation: 0, lightness };
  const saturation = delta / (1 - Math.abs(2 * lightness - 1));
  let hue: number;
  if (max === r) hue = ((g - b) / delta) % 6;
  else if (max === g) hue = (b - r) / delta + 2;
  else hue = (r - g) / delta + 4;
  return { hue: (hue * 60 + 360) % 360, saturation, lightness };
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

/**
 * Average color weighted toward vivid pixels, then pushed into a range that
 * glows nicely: a grey or black cover still yields a soft, visible light.
 */
export function glowFromPixels(pixels: Uint8ClampedArray): string {
  let red = 0;
  let green = 0;
  let blue = 0;
  let totalWeight = 0;
  for (let index = 0; index < pixels.length; index += 4) {
    const r = pixels[index] ?? 0;
    const g = pixels[index + 1] ?? 0;
    const b = pixels[index + 2] ?? 0;
    const { saturation, lightness } = rgbToHsl(r, g, b);
    const usable = lightness > 0.12 && lightness < 0.92;
    const weight = (usable ? saturation * saturation : 0.02) + 0.001;
    red += r * weight;
    green += g * weight;
    blue += b * weight;
    totalWeight += weight;
  }
  const average = rgbToHsl(red / totalWeight, green / totalWeight, blue / totalWeight);
  const saturation = clamp(average.saturation, MIN_SATURATION, MAX_SATURATION);
  const lightness = clamp(average.lightness, MIN_LIGHTNESS, MAX_LIGHTNESS);
  return `hsl(${Math.round(average.hue)} ${Math.round(saturation * 100)}% ${Math.round(lightness * 100)}%)`;
}

async function extractGlowColor(source: string): Promise<string> {
  const image = new Image();
  image.src = source;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = SAMPLE_SIZE;
  canvas.height = SAMPLE_SIZE;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("Canvas 2D is unavailable");
  context.drawImage(image, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
  return glowFromPixels(context.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE).data);
}

function remember(source: string, color: string): void {
  glowCache.set(source, color);
  if (glowCache.size > MAX_CACHED_COLORS) {
    const oldest = glowCache.keys().next().value;
    if (oldest !== undefined) glowCache.delete(oldest);
  }
}

/** Glow color of the current cover, or null to fall back to the accent. */
export function useArtworkGlow(artwork: string | null, enabled: boolean): string | null {
  const [color, setColor] = useState<string | null>(null);

  useEffect(() => {
    if (!artwork || !enabled) {
      setColor(null);
      return;
    }
    const cached = glowCache.get(artwork);
    if (cached) {
      setColor(cached);
      return;
    }
    let alive = true;
    extractGlowColor(artwork)
      .then((extracted) => {
        remember(artwork, extracted);
        if (alive) setColor(extracted);
      })
      .catch((error: unknown) => {
        console.warn("Cannot read the cover colors", error);
        if (alive) setColor(null);
      });
    return () => {
      alive = false;
    };
  }, [artwork, enabled]);

  return color;
}
