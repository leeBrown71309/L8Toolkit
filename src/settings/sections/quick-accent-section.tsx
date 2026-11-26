import { Check } from "@phosphor-icons/react";
import { useEffect, useState } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend } from "../../shared/backend";
import { Keycap } from "../../shared/keycap";
import type { AccentLanguage, AccentPreviewRow, AccentTrigger, QuickAccentSettings, SettingsSnapshot } from "../../shared/types";
import { SectionHeader, Segmented, SettingRow, SettingsGroup, Toggle } from "../components/controls";
import type { SettingsController } from "../use-settings-controller";

/** Each language under its own name, so anyone finds theirs. */
const LANGUAGES: { code: AccentLanguage; name: string }[] = [
  { code: "fr", name: "Français" },
  { code: "es", name: "Español" },
  { code: "pt", name: "Português" },
  { code: "de", name: "Deutsch" },
  { code: "it", name: "Italiano" },
  { code: "nl", name: "Nederlands" },
  { code: "ca", name: "Català" },
  { code: "tr", name: "Türkçe" },
  { code: "pl", name: "Polski" },
  { code: "ro", name: "Română" },
  { code: "cs", name: "Čeština" },
  { code: "sk", name: "Slovenčina" },
  { code: "hu", name: "Magyar" },
  { code: "hr", name: "Hrvatski · Srpski · Bosanski" },
  { code: "sl", name: "Slovenščina" },
  { code: "nordic", name: "Dansk · Norsk · Svenska" },
  { code: "fi", name: "Suomi" },
  { code: "is", name: "Íslenska" },
  { code: "et", name: "Eesti" },
  { code: "lv", name: "Latviešu" },
  { code: "lt", name: "Lietuvių" },
  { code: "mt", name: "Malti" },
  { code: "cy", name: "Cymraeg" },
  { code: "ga", name: "Gaeilge · Gàidhlig" },
  { code: "sq", name: "Shqip" },
  { code: "ku", name: "Kurdî" },
  { code: "eo", name: "Esperanto" },
  { code: "vi", name: "Tiếng Việt" },
  { code: "mi", name: "Te Reo Māori" },
  { code: "pinyin", name: "Pīnyīn" },
];

/** Sets that are not languages; their names are translated. */
const EXTRA_SETS: { code: AccentLanguage; labelKey: "quickAccent.set.currency" | "quickAccent.set.symbols" }[] = [
  { code: "currency", labelKey: "quickAccent.set.currency" },
  { code: "symbols", labelKey: "quickAccent.set.symbols" },
];

interface QuickAccentSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

export function QuickAccentSection({ snapshot, controller }: QuickAccentSectionProps) {
  const { t } = useI18n();
  const settings = snapshot.settings.quickAccent;
  const preview = useAccentPreview(settings.languages);
  const update = (patch: Partial<QuickAccentSettings>) =>
    controller.updateSettings((current) => ({ ...current, quickAccent: { ...current.quickAccent, ...patch } }));

  // Selecting appends, so the order of the clicks is the priority order.
  const toggleLanguage = (code: AccentLanguage) => {
    const languages = settings.languages.includes(code)
      ? settings.languages.filter((language) => language !== code)
      : [...settings.languages, code];
    update({ languages });
  };

  const triggers: { value: AccentTrigger; label: string }[] = [
    { value: "spaceAndArrows", label: t("quickAccent.trigger.spaceAndArrows") },
    { value: "space", label: t("quickAccent.trigger.space") },
    { value: "arrows", label: t("quickAccent.trigger.arrows") },
  ];

  return (
    <>
      <SectionHeader title={t("quickAccent.title")} description={t("quickAccent.description")} />

      <SettingsGroup title={t("quickAccent.activation")}>
        <SettingRow
          label={t("quickAccent.enable")}
          description={t("quickAccent.enable.description")}
          control={<Toggle label={t("quickAccent.enable")} checked={settings.enabled} onChange={(enabled) => update({ enabled })} />}
        />
        <SettingRow
          label={t("quickAccent.trigger")}
          description={t("quickAccent.trigger.description")}
          disabled={!settings.enabled}
          control={
            <Segmented label={t("quickAccent.trigger")} value={settings.trigger} options={triggers} onChange={(trigger) => update({ trigger })} />
          }
        />
        <SettingRow label={t("quickAccent.howTo")} disabled={!settings.enabled}>
          <ol className="accent-steps">
            <li>
              <Keycap>e</Keycap>
              <span>{t("quickAccent.howTo.hold")}</span>
            </li>
            <li>
              <Keycap>{t("keys.space")}</Keycap>
              <Keycap>←</Keycap>
              <Keycap>→</Keycap>
              <span>{t("quickAccent.howTo.choose")}</span>
            </li>
            <li>
              <span>{t("quickAccent.howTo.release")}</span>
            </li>
          </ol>
          <input className="accent-try" type="text" placeholder={t("quickAccent.try")} aria-label={t("quickAccent.try")} spellCheck={false} />
        </SettingRow>
      </SettingsGroup>

      <SettingsGroup title={t("quickAccent.languages")}>
        <SettingRow label={t("quickAccent.languages.label")} description={t("quickAccent.languages.description")} disabled={!settings.enabled}>
          <div className="accent-languages">
            <button
              type="button"
              className="accent-language"
              aria-pressed={settings.languages.length === 0}
              onClick={() => update({ languages: [] })}
            >
              {settings.languages.length === 0 && <Check size={13} weight="bold" />}
              {t("quickAccent.languages.all")}
            </button>
            {[...LANGUAGES, ...EXTRA_SETS.map(({ code, labelKey }) => ({ code, name: t(labelKey) }))].map(({ code, name }) => {
              const position = settings.languages.indexOf(code);
              return (
                <button
                  key={code}
                  type="button"
                  className="accent-language"
                  aria-pressed={position !== -1}
                  onClick={() => toggleLanguage(code)}
                >
                  {position !== -1 && settings.languages.length > 1 && <span className="accent-language__rank">{position + 1}</span>}
                  {position !== -1 && settings.languages.length === 1 && <Check size={13} weight="bold" />}
                  {name}
                </button>
              );
            })}
          </div>
        </SettingRow>
        <SettingRow label={t("quickAccent.preview")} disabled={!settings.enabled}>
          <div className="accent-preview">
            {preview.map(({ letter, characters }) => (
              <div key={letter} className="accent-preview__row">
                <span className="accent-preview__letter">{letter}</span>
                <span className="accent-preview__characters">{characters.join(" ")}</span>
              </div>
            ))}
          </div>
        </SettingRow>
      </SettingsGroup>
    </>
  );
}

/** Letters and accents offered by the selected languages (computed in Rust). */
function useAccentPreview(languages: AccentLanguage[]): AccentPreviewRow[] {
  const [rows, setRows] = useState<AccentPreviewRow[]>([]);
  const key = languages.join(",");
  useEffect(() => {
    let alive = true;
    backend
      .quickAccentPreview(key ? (key.split(",") as AccentLanguage[]) : [])
      .then((next) => {
        if (alive) setRows(next);
      })
      .catch((error: unknown) => console.error("Cannot list the accents", error));
    return () => {
      alive = false;
    };
  }, [key]);
  return rows;
}
