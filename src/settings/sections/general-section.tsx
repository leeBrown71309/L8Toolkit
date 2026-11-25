import { Power } from "@phosphor-icons/react";

import { useI18n } from "../../i18n/i18n-context";
import type { LanguagePreference, SettingsSnapshot } from "../../shared/types";
import { SectionHeader, Segmented, SettingRow, SettingsGroup, Toggle } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

interface GeneralSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

export function GeneralSection({ snapshot, controller }: GeneralSectionProps) {
  const { t } = useI18n();
  const { settings, status } = snapshot;
  // Language names are written in their own language, so anyone can find theirs.
  const languages: { value: LanguagePreference; label: string }[] = [
    { value: "system", label: t("general.language.system") },
    { value: "en", label: "English" },
    { value: "fr", label: "Français" },
  ];

  return (
    <>
      <SectionHeader title={t("general.title")} description={t("general.description")} />

      <SettingsGroup title={t("general.language")}>
        <SettingRow
          label={t("general.language.label")}
          description={t("general.language.description")}
          control={
            <Segmented
              label={t("general.language.label")}
              value={settings.language}
              options={languages}
              onChange={(language) => controller.updateSettings((current) => ({ ...current, language }))}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("general.startup")}>
        <SettingRow
          label={t("general.launchAtStartup")}
          description={t("general.launchAtStartup.description")}
          control={
            <Toggle
              label={t("general.launchAtStartup")}
              checked={settings.launchAtStartup}
              onChange={(launchAtStartup) => controller.updateSettings((current) => ({ ...current, launchAtStartup }))}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("general.application")}>
        <SettingRow label={t("general.version")} description={`L8 Toolkit ${status.version}`} />
        <SettingRow
          label={t("general.quit")}
          description={t("general.quit.description")}
          control={
            <button type="button" className="button" onClick={controller.quitApp}>
              <Power size={15} />
              {t("general.quit.button")}
            </button>
          }
        />
      </SettingsGroup>
    </>
  );
}
