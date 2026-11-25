import { getCurrentWindow } from "@tauri-apps/api/window";
import { lazy, Suspense, type ReactNode } from "react";

import { guessLocale, I18nProvider } from "../i18n/i18n-context";
import { useSettingsSnapshot } from "../shared/appearance";
import { isTauriRuntime } from "../shared/backend";

export type WindowKind = "notch" | "settings";

// Each window only downloads the code it renders.
const NotchApp = lazy(() => import("../widgets/notch/notch-app").then((module) => ({ default: module.NotchApp })));
const SettingsApp = lazy(() => import("../settings/settings-app").then((module) => ({ default: module.SettingsApp })));

/** Same bundle, two windows: the Tauri window label decides what to render. */
export function resolveWindowKind(): WindowKind {
  const label = isTauriRuntime
    ? getCurrentWindow().label
    : new URLSearchParams(window.location.search).get("window");
  return label === "settings" ? "settings" : "notch";
}

/** Both windows speak the language chosen in the settings, live. */
function LanguageBoundary({ children }: { children: ReactNode }) {
  const snapshot = useSettingsSnapshot();
  return <I18nProvider locale={snapshot?.effectiveLanguage ?? guessLocale()}>{children}</I18nProvider>;
}

export function WindowRouter({ windowKind }: { windowKind: WindowKind }) {
  return (
    <LanguageBoundary>
      <Suspense fallback={null}>{windowKind === "settings" ? <SettingsApp /> : <NotchApp />}</Suspense>
    </LanguageBoundary>
  );
}
