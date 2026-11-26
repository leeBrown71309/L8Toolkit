import {
  ArrowsClockwise,
  type Icon,
  MagnifyingGlass,
  MusicNotesSimple,
  Palette,
  SlidersHorizontal,
  TextAa,
  WarningCircle,
} from "@phosphor-icons/react";
import { useState } from "react";

import appIcon from "../assets/app-icon.png";
import { useI18n } from "../i18n/i18n-context";
import type { MessageKey } from "../i18n/messages/en";
import { useApplyAppearance } from "../shared/appearance";
import { BACKEND_EVENTS } from "../shared/backend";
import { useBackendEvent } from "../shared/use-backend-event";
import { AppearanceSection } from "./sections/appearance-section";
import { GeneralSection } from "./sections/general-section";
import { MusicSection } from "./sections/music-section";
import { QuickAccentSection } from "./sections/quick-accent-section";
import { SpotlightSection } from "./sections/spotlight-section";
import { UpdatesSection } from "./sections/updates-section";
import { useSettingsController } from "./use-settings-controller";
import { useUpdateState } from "./use-update-state";

import "./settings.css";

type SectionKey = "general" | "spotlight" | "appearance" | "music" | "quickAccent" | "updates";

interface NavigationItem {
  key: SectionKey;
  labelKey: MessageKey;
  icon: Icon;
}

const NAVIGATION: NavigationItem[] = [
  { key: "spotlight", labelKey: "nav.spotlight", icon: MagnifyingGlass },
  { key: "appearance", labelKey: "nav.appearance", icon: Palette },
  { key: "music", labelKey: "nav.music", icon: MusicNotesSimple },
  { key: "quickAccent", labelKey: "nav.quickAccent", icon: TextAa },
  { key: "general", labelKey: "nav.general", icon: SlidersHorizontal },
  { key: "updates", labelKey: "nav.updates", icon: ArrowsClockwise },
];

const SECTION_KEYS = new Set<string>(NAVIGATION.map(({ key }) => key));

function isSectionKey(value: unknown): value is SectionKey {
  return typeof value === "string" && SECTION_KEYS.has(value);
}

/** The tray opens the window on a section with `?section=updates`. */
function initialSection(): SectionKey {
  const requested = new URLSearchParams(window.location.search).get("section");
  return isSectionKey(requested) ? requested : "spotlight";
}

export function SettingsApp() {
  const { t } = useI18n();
  const controller = useSettingsController();
  const [section, setSection] = useState<SectionKey>(initialSection);
  const updates = useUpdateState();
  const updatePending = updates.state.status === "available";
  // Already open: the tray asks for a section through an event instead.
  useBackendEvent<string>(BACKEND_EVENTS.settingsNavigate, (requested) => {
    if (isSectionKey(requested)) setSection(requested);
  });
  const { snapshot, error } = controller;
  useApplyAppearance(snapshot?.settings.appearance, snapshot?.systemAccentColor ?? null);

  return (
    <div className="settings">
      <aside className="settings-nav">
        <div className="settings-brand">
          <img src={appIcon} width={34} height={34} alt="" />
          <div>
            <div className="settings-brand__name">L8 Toolkit</div>
            <div className="settings-brand__meta">
              {snapshot?.status.devBuild ? t("settings.devBuild") : t("settings.tagline")}
            </div>
          </div>
        </div>
        <nav className="settings-nav__items" aria-label={t("settings.sections")}>
          {NAVIGATION.map(({ key, labelKey, icon: Glyph }) => (
            <button
              key={key}
              type="button"
              className="settings-nav__item"
              aria-current={section === key ? "page" : undefined}
              onClick={() => setSection(key)}
            >
              <Glyph size={17} />
              {t(labelKey)}
              {key === "updates" && updatePending && (
                <span className="settings-nav__dot" role="status" aria-label={t("updates.badge")} />
              )}
            </button>
          ))}
        </nav>
      </aside>

      <main className="settings-main">
        <div className="settings-main__inner">
          {error && (
            <div className="settings-banner" role="alert">
              <WarningCircle size={16} weight="fill" />
              {error}
            </div>
          )}
          {!snapshot ? (
            <div className="settings-skeleton" aria-busy="true" aria-label={t("settings.loading")}>
              <div className="settings-skeleton__title" />
              <div className="settings-skeleton__group" />
              <div className="settings-skeleton__group" />
            </div>
          ) : (
            <>
              {section === "spotlight" && (
                <SpotlightSection snapshot={snapshot} controller={controller} />
              )}
              {section === "appearance" && <AppearanceSection snapshot={snapshot} controller={controller} />}
              {section === "music" && <MusicSection snapshot={snapshot} controller={controller} />}
              {section === "quickAccent" && <QuickAccentSection snapshot={snapshot} controller={controller} />}
              {section === "general" && <GeneralSection snapshot={snapshot} controller={controller} />}
              {section === "updates" && <UpdatesSection snapshot={snapshot} updates={updates} />}
            </>
          )}
        </div>
      </main>
    </div>
  );
}
