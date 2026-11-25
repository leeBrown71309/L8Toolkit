import { useCallback, useEffect, useRef, useState } from "react";

import { useI18n } from "../i18n/i18n-context";
import { backend, BACKEND_EVENTS, describeError } from "../shared/backend";
import type { AppSettings, AppStatus, SettingsSnapshot, SpotlightSettings } from "../shared/types";
import { useBackendEvent } from "../shared/use-backend-event";

export interface SettingsController {
  snapshot: SettingsSnapshot | null;
  error: string | null;
  updateSettings: (mutate: (settings: AppSettings) => AppSettings) => void;
  updateSpotlight: (patch: Partial<SpotlightSettings>) => void;
  rebuildIndex: () => void;
  quitApp: () => void;
  reportError: (message: string) => void;
}

/**
 * Loads settings, applies changes optimistically and rolls back when Rust
 * rejects them, so toggles feel instant but never lie about the real state.
 */
export function useSettingsController(): SettingsController {
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<SettingsSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const snapshotRef = useRef<SettingsSnapshot | null>(null);

  useEffect(() => {
    snapshotRef.current = snapshot;
  });

  useEffect(() => {
    backend
      .getSettings()
      .then(setSnapshot)
      .catch((loadError: unknown) => setError(t("settings.loadFailed", { reason: describeError(loadError) })));
  }, []);

  useBackendEvent<SettingsSnapshot>(BACKEND_EVENTS.settingsChanged, setSnapshot);
  useBackendEvent<AppStatus>(BACKEND_EVENTS.indexStatus, (status) =>
    setSnapshot((current) => (current ? { ...current, status } : current)),
  );

  const updateSettings = useCallback((mutate: (settings: AppSettings) => AppSettings) => {
    const previous = snapshotRef.current;
    if (!previous) return;
    const next = mutate(previous.settings);
    setSnapshot({ ...previous, settings: next });
    setError(null);
    backend
      .updateSettings(next)
      .then(setSnapshot)
      .catch((saveError: unknown) => {
        setSnapshot(previous);
        setError(describeError(saveError));
      });
  }, []);

  const updateSpotlight = useCallback(
    (patch: Partial<SpotlightSettings>) =>
      updateSettings((settings) => ({ ...settings, spotlight: { ...settings.spotlight, ...patch } })),
    [updateSettings],
  );

  const rebuildIndex = useCallback(() => {
    backend.rebuildIndex().catch((rebuildError: unknown) => setError(describeError(rebuildError)));
  }, []);

  const quitApp = useCallback(() => {
    backend.quitApp().catch((quitError: unknown) => setError(describeError(quitError)));
  }, []);

  return { snapshot, error, updateSettings, updateSpotlight, rebuildIndex, quitApp, reportError: setError };
}
