import { useI18n } from "../../i18n/i18n-context";
import type { MusicSettings, SettingsSnapshot } from "../../shared/types";
import { SectionHeader, SettingRow, SettingsGroup, Toggle } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

interface MusicSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

export function MusicSection({ snapshot, controller }: MusicSectionProps) {
  const { t } = useI18n();
  const { music } = snapshot.settings;
  const update = (patch: Partial<MusicSettings>) =>
    controller.updateSettings((settings) => ({ ...settings, music: { ...settings.music, ...patch } }));

  return (
    <>
      <SectionHeader title={t("musicSettings.title")} description={t("musicSettings.description")} />

      <SettingsGroup title={t("musicSettings.widget")}>
        <SettingRow
          label={t("musicSettings.enable")}
          description={t("musicSettings.enable.description")}
          control={<Toggle label={t("musicSettings.enable")} checked={music.enabled} onChange={(enabled) => update({ enabled })} />}
        />
      </SettingsGroup>

      <SettingsGroup title={t("musicSettings.inNotch")}>
        <SettingRow
          label={t("musicSettings.live")}
          description={t("musicSettings.live.description")}
          disabled={!music.enabled}
          control={
            <Toggle
              label={t("musicSettings.live")}
              checked={music.showInNotch}
              disabled={!music.enabled}
              onChange={(showInNotch) => update({ showInNotch })}
            />
          }
        />
        <SettingRow
          label={t("musicSettings.audioBars")}
          description={t("musicSettings.audioBars.description")}
          disabled={!music.enabled || !music.showInNotch}
          control={
            <Toggle
              label={t("musicSettings.audioBars")}
              checked={music.showAudioBars}
              disabled={!music.enabled || !music.showInNotch}
              onChange={(showAudioBars) => update({ showAudioBars })}
            />
          }
        />
        <SettingRow
          label={t("musicSettings.peek")}
          description={t("musicSettings.peek.description")}
          disabled={!music.enabled || !music.showInNotch}
          control={
            <Toggle
              label={t("musicSettings.peek")}
              checked={music.peekOnTrackChange}
              disabled={!music.enabled || !music.showInNotch}
              onChange={(peekOnTrackChange) => update({ peekOnTrackChange })}
            />
          }
        />
        <SettingRow
          label={t("musicSettings.tint")}
          description={t("musicSettings.tint.description")}
          disabled={!music.enabled}
          control={
            <Toggle
              label={t("musicSettings.tint")}
              checked={music.artworkTint}
              disabled={!music.enabled}
              onChange={(artworkTint) => update({ artworkTint })}
            />
          }
        />
      </SettingsGroup>
    </>
  );
}
