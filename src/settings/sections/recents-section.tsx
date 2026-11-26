import { useEffect, useState } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend } from "../../shared/backend";
import type { RecentsSettings, SettingsSnapshot } from "../../shared/types";
import { SectionHeader, SettingRow, SettingsGroup, Stepper, Toggle } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

interface RecentsSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

/** Same bounds as RECENTS_MIN_LIMIT / RECENTS_MAX_LIMIT in settings.rs. */
const MIN_LIMIT = 3;
const MAX_LIMIT = 50;
const CLEARED_FEEDBACK_MS = 2200;

export function RecentsSection({ snapshot, controller }: RecentsSectionProps) {
  const { t } = useI18n();
  const { recents } = snapshot.settings;
  const [cleared, setCleared] = useState(false);
  const update = (patch: Partial<RecentsSettings>) =>
    controller.updateSettings((settings) => ({ ...settings, recents: { ...settings.recents, ...patch } }));

  useEffect(() => {
    if (!cleared) return;
    const timer = window.setTimeout(() => setCleared(false), CLEARED_FEEDBACK_MS);
    return () => window.clearTimeout(timer);
  }, [cleared]);

  const clearAll = () => {
    backend
      .recentsClear()
      .then(() => setCleared(true))
      .catch((error: unknown) => console.error("Cannot clear Recents", error));
  };

  return (
    <>
      <SectionHeader title={t("recentsSettings.title")} description={t("recentsSettings.description")} />

      <SettingsGroup title={t("recentsSettings.widget")}>
        <SettingRow
          label={t("recentsSettings.enable")}
          description={t("recentsSettings.enable.description")}
          control={<Toggle label={t("recentsSettings.enable")} checked={recents.enabled} onChange={(enabled) => update({ enabled })} />}
        />
        <SettingRow
          label={t("recentsSettings.limit")}
          description={t("recentsSettings.limit.description")}
          disabled={!recents.enabled}
          control={
            <Stepper
              label={t("recentsSettings.limit")}
              value={recents.limit}
              min={MIN_LIMIT}
              max={MAX_LIMIT}
              unit={t("recentsSettings.items")}
              disabled={!recents.enabled}
              decreaseLabel={t("timerSettings.decrease")}
              increaseLabel={t("timerSettings.increase")}
              onChange={(limit) => update({ limit })}
            />
          }
        />
        <SettingRow
          label={t("recentsSettings.screenshotPeek")}
          description={t("recentsSettings.screenshotPeek.description")}
          disabled={!recents.enabled}
          control={
            <Toggle
              label={t("recentsSettings.screenshotPeek")}
              checked={recents.screenshotPeek}
              disabled={!recents.enabled}
              onChange={(screenshotPeek) => update({ screenshotPeek })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("recentsSettings.privacy")}>
        <SettingRow
          label={t("recentsSettings.passwordManagers")}
          description={t("recentsSettings.passwordManagers.description")}
          disabled={!recents.enabled}
          control={
            <Toggle
              label={t("recentsSettings.passwordManagers")}
              checked={recents.keepPasswordManagerItems}
              disabled={!recents.enabled}
              onChange={(keepPasswordManagerItems) => update({ keepPasswordManagerItems })}
            />
          }
        />
        <SettingRow
          label={t("recentsSettings.clear")}
          description={t("recentsSettings.clear.description")}
          control={
            <button type="button" className="button" disabled={cleared} onClick={clearAll}>
              {cleared ? t("recentsSettings.clear.done") : t("recentsSettings.clear.button")}
            </button>
          }
        />
      </SettingsGroup>
    </>
  );
}
