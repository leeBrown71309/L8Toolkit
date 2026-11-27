import { useI18n } from "../../i18n/i18n-context";
import type { MessageKey } from "../../i18n/messages/en";
import type { SettingsSnapshot, TimerSettings } from "../../shared/types";
import { SectionHeader, SettingRow, SettingsGroup, Stepper, Toggle } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

interface TimerSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

type MinutesField = "focusMinutes" | "shortBreakMinutes" | "longBreakMinutes";

/** Same bounds as `TimerSettings::sanitized` in settings.rs. */
const MINUTE_FIELDS: { field: MinutesField; labelKey: MessageKey; max: number }[] = [
  { field: "focusMinutes", labelKey: "timerSettings.focus", max: 180 },
  { field: "shortBreakMinutes", labelKey: "timerSettings.shortBreak", max: 60 },
  { field: "longBreakMinutes", labelKey: "timerSettings.longBreak", max: 120 },
];
const MAX_ROUNDS = 12;

export function TimerSection({ snapshot, controller }: TimerSectionProps) {
  const { t } = useI18n();
  const { timer } = snapshot.settings;
  const update = (patch: Partial<TimerSettings>) =>
    controller.updateSettings((settings) => ({ ...settings, timer: { ...settings.timer, ...patch } }));
  const stepperLabels = { decreaseLabel: t("timerSettings.decrease"), increaseLabel: t("timerSettings.increase") };

  return (
    <>
      <SectionHeader title={t("timerSettings.title")} description={t("timerSettings.description")} />

      <SettingsGroup title={t("timerSettings.widget")}>
        <SettingRow
          label={t("timerSettings.enable")}
          description={t("timerSettings.enable.description")}
          control={<Toggle label={t("timerSettings.enable")} checked={timer.enabled} onChange={(enabled) => update({ enabled })} />}
        />
      </SettingsGroup>

      <SettingsGroup title={t("timerSettings.inNotch")}>
        <SettingRow
          label={t("timerSettings.live")}
          description={t("timerSettings.live.description")}
          disabled={!timer.enabled}
          control={
            <Toggle
              label={t("timerSettings.live")}
              checked={timer.showInNotch}
              disabled={!timer.enabled}
              onChange={(showInNotch) => update({ showInNotch })}
            />
          }
        />
        <SettingRow
          label={t("timerSettings.notify")}
          description={t("timerSettings.notify.description")}
          disabled={!timer.enabled}
          control={
            <Toggle
              label={t("timerSettings.notify")}
              checked={timer.notify}
              disabled={!timer.enabled}
              onChange={(notify) => update({ notify })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("timerSettings.pomodoro")}>
        {MINUTE_FIELDS.map(({ field, labelKey, max }) => (
          <SettingRow
            key={field}
            label={t(labelKey)}
            disabled={!timer.enabled}
            control={
              <Stepper
                label={t(labelKey)}
                value={timer[field]}
                min={1}
                max={max}
                unit={t("timerSettings.minutes")}
                disabled={!timer.enabled}
                {...stepperLabels}
                onChange={(value) => update({ [field]: value })}
              />
            }
          />
        ))}
        <SettingRow
          label={t("timerSettings.rounds")}
          disabled={!timer.enabled}
          control={
            <Stepper
              label={t("timerSettings.rounds")}
              value={timer.roundsBeforeLongBreak}
              min={1}
              max={MAX_ROUNDS}
              disabled={!timer.enabled}
              {...stepperLabels}
              onChange={(roundsBeforeLongBreak) => update({ roundsBeforeLongBreak })}
            />
          }
        />
        <SettingRow
          label={t("timerSettings.autoContinue")}
          description={t("timerSettings.autoContinue.description")}
          disabled={!timer.enabled}
          control={
            <Toggle
              label={t("timerSettings.autoContinue")}
              checked={timer.autoContinue}
              disabled={!timer.enabled}
              onChange={(autoContinue) => update({ autoContinue })}
            />
          }
        />
      </SettingsGroup>
    </>
  );
}
