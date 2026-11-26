import { useEffect, useState } from "react";

import type { MessageKey } from "../i18n/messages/en";
import { backend, BACKEND_EVENTS } from "./backend";
import type { AccentColor, AppearanceSettings, SettingsSnapshot } from "./types";
import { useBackendEvent } from "./use-backend-event";

type PresetAccent = Exclude<AccentColor, "system">;

/** Picked to stay legible as text on both the dark and light themes. */
export const ACCENT_PRESETS: Record<PresetAccent, { labelKey: MessageKey; value: string }> = {
  ember: { labelKey: "accent.ember", value: "#ec9a55" },
  ocean: { labelKey: "accent.ocean", value: "#4c9af0" },
  jade: { labelKey: "accent.jade", value: "#3dbb8c" },
  rose: { labelKey: "accent.rose", value: "#ec6f8e" },
  iris: { labelKey: "accent.iris", value: "#8f86f2" },
  graphite: { labelKey: "accent.graphite", value: "#a3a3ad" },
};

const FALLBACK_ACCENT = ACCENT_PRESETS.ember.value;
const LIGHT_SCHEME_QUERY = "(prefers-color-scheme: light)";

/** Relative luminance of a #rrggbb color (WCAG). */
function luminance(hex: string): number {
  const channel = (offset: number) => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

const INK_DARK = "#0b0b0d";
const INK_LIGHT = "#ffffff";

/** Text color for content drawn on the accent: whichever contrasts more. */
export function inkForAccent(accent: string): string {
  if (!/^#[0-9a-f]{6}$/i.test(accent)) return INK_LIGHT;
  const background = luminance(accent);
  const contrastWithDark = (background + 0.05) / (luminance(INK_DARK) + 0.05);
  const contrastWithLight = (1.05) / (background + 0.05);
  return contrastWithDark >= contrastWithLight ? INK_DARK : INK_LIGHT;
}

export function resolveAccent(accent: AccentColor, systemAccent: string | null): string {
  if (accent === "system") return systemAccent ?? FALLBACK_ACCENT;
  return ACCENT_PRESETS[accent].value;
}

/** "system" follows Windows; the window theme set by Rust drives the media query. */
function usePrefersLight(): boolean {
  const [prefersLight, setPrefersLight] = useState(() => window.matchMedia(LIGHT_SCHEME_QUERY).matches);
  useEffect(() => {
    const query = window.matchMedia(LIGHT_SCHEME_QUERY);
    const update = () => setPrefersLight(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  return prefersLight;
}

/** Reflects the appearance settings on <html> for the CSS tokens. */
export function useApplyAppearance(appearance: AppearanceSettings | undefined, systemAccent: string | null): void {
  const prefersLight = usePrefersLight();

  useEffect(() => {
    if (!appearance) return;
    const root = document.documentElement;
    const light = appearance.theme === "light" || (appearance.theme === "system" && prefersLight);
    root.dataset.theme = light ? "light" : "dark";
    root.dataset.material = appearance.material;
    const accent = resolveAccent(appearance.accent, systemAccent);
    root.style.setProperty("--accent", accent);
    root.style.setProperty("--accent-ink", inkForAccent(accent));
  }, [appearance, systemAccent, prefersLight]);
}

/** Settings for windows that only need to read them (the notch). */
export function useSettingsSnapshot(): SettingsSnapshot | null {
  const [snapshot, setSnapshot] = useState<SettingsSnapshot | null>(null);

  useEffect(() => {
    backend
      .getSettings()
      .then(setSnapshot)
      .catch((error: unknown) => console.error("Cannot load settings", error));
  }, []);

  useBackendEvent<SettingsSnapshot>(BACKEND_EVENTS.settingsChanged, setSnapshot);
  return snapshot;
}
