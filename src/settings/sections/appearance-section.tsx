import { Check } from "@phosphor-icons/react";

import { useI18n } from "../../i18n/i18n-context";
import type { MessageKey } from "../../i18n/messages/en";
import { ACCENT_PRESETS, resolveAccent } from "../../shared/appearance";
import type { AccentColor, AppearanceSettings, Material, SettingsSnapshot, ThemeMode } from "../../shared/types";
import { SectionHeader, Segmented, SettingRow, SettingsGroup } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

const THEMES: { value: ThemeMode; labelKey: MessageKey }[] = [
  { value: "system", labelKey: "appearance.theme.system" },
  { value: "dark", labelKey: "appearance.theme.dark" },
  { value: "light", labelKey: "appearance.theme.light" },
];

const MATERIALS: { value: Material; labelKey: MessageKey; descriptionKey: MessageKey }[] = [
  { value: "solid", labelKey: "appearance.material.solid", descriptionKey: "appearance.material.solid.description" },
  {
    value: "translucent",
    labelKey: "appearance.material.translucent",
    descriptionKey: "appearance.material.translucent.description",
  },
  { value: "mica", labelKey: "appearance.material.mica", descriptionKey: "appearance.material.mica.description" },
  { value: "acrylic", labelKey: "appearance.material.acrylic", descriptionKey: "appearance.material.acrylic.description" },
];

const ACCENTS: { value: AccentColor; labelKey: MessageKey }[] = [
  ...Object.entries(ACCENT_PRESETS).map(([value, preset]) => ({ value: value as AccentColor, labelKey: preset.labelKey })),
  { value: "system", labelKey: "accent.system" },
];

interface AppearanceSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

export function AppearanceSection({ snapshot, controller }: AppearanceSectionProps) {
  const { t } = useI18n();
  const { appearance } = snapshot.settings;
  const update = (patch: Partial<AppearanceSettings>) =>
    controller.updateSettings((settings) => ({ ...settings, appearance: { ...settings.appearance, ...patch } }));

  return (
    <>
      <SectionHeader title={t("appearance.title")} description={t("appearance.description")} />

      <SettingsGroup title={t("appearance.theme")}>
        <SettingRow
          label={t("appearance.mode")}
          description={t("appearance.mode.description")}
          control={
            <Segmented
              label={t("appearance.theme")}
              value={appearance.theme}
              options={THEMES.map(({ value, labelKey }) => ({ value, label: t(labelKey) }))}
              onChange={(theme) => update({ theme })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("appearance.material")}>
        <SettingRow label={t("appearance.material.label")} description={t("appearance.material.description")}>
          <div className="material-grid" role="radiogroup" aria-label={t("appearance.material.label")}>
            {MATERIALS.map((material) => (
              <button
                key={material.value}
                type="button"
                role="radio"
                aria-checked={appearance.material === material.value}
                className="material-card"
                onClick={() => update({ material: material.value })}
              >
                <span className="material-card__preview" aria-hidden="true">
                  <span className="material-card__sample" data-material={material.value} />
                </span>
                <span className="material-card__label">{t(material.labelKey)}</span>
                <span className="material-card__description">{t(material.descriptionKey)}</span>
              </button>
            ))}
          </div>
        </SettingRow>
      </SettingsGroup>

      <SettingsGroup title={t("appearance.accent")}>
        <SettingRow label={t("appearance.accent.label")} description={t("appearance.accent.description")}>
          <div className="accent-grid" role="radiogroup" aria-label={t("appearance.accent")}>
            {ACCENTS.map((accent) => {
              const selected = appearance.accent === accent.value;
              return (
                <button
                  key={accent.value}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  className="accent-swatch"
                  onClick={() => update({ accent: accent.value })}
                >
                  <span
                    className="accent-swatch__color"
                    style={{ background: resolveAccent(accent.value, snapshot.systemAccentColor) }}
                  >
                    {selected && <Check size={14} weight="bold" />}
                  </span>
                  <span className="accent-swatch__label">{t(accent.labelKey)}</span>
                </button>
              );
            })}
          </div>
        </SettingRow>
      </SettingsGroup>
    </>
  );
}
