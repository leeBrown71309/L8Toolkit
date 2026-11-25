import { ArrowsClockwise, FolderPlus, X } from "@phosphor-icons/react";
import { open } from "@tauri-apps/plugin-dialog";

import { useI18n } from "../../i18n/i18n-context";
import { describeError, isTauriRuntime } from "../../shared/backend";
import type { SettingsSnapshot, WebSearchEngine } from "../../shared/types";
import { SectionHeader, Segmented, SettingRow, SettingsGroup, Toggle } from "../components/controls";
import { HotkeyRecorder } from "../components/hotkey-recorder";
import type { SettingsController } from "../use-settings-controller";

/** Brand names: identical in every language. */
const SEARCH_ENGINES: { value: WebSearchEngine; label: string }[] = [
  { value: "google", label: "Google" },
  { value: "bing", label: "Bing" },
  { value: "duckduckgo", label: "DuckDuckGo" },
  { value: "brave", label: "Brave" },
];

interface SpotlightSectionProps {
  snapshot: SettingsSnapshot;
  controller: SettingsController;
}

export function SpotlightSection({ snapshot, controller }: SpotlightSectionProps) {
  const { t, locale } = useI18n();
  const { spotlight } = snapshot.settings;
  const { status } = snapshot;
  const { updateSpotlight } = controller;
  const numberFormat = new Intl.NumberFormat(locale);

  const addFolder = async () => {
    if (!isTauriRuntime) return;
    try {
      const picked = await open({ directory: true, multiple: false, title: t("spotlight.folders.pickerTitle") });
      if (typeof picked === "string" && !spotlight.extraFolders.includes(picked)) {
        updateSpotlight({ extraFolders: [...spotlight.extraFolders, picked] });
      }
    } catch (error) {
      controller.reportError(t("spotlight.folders.pickerFailed", { reason: describeError(error) }));
    }
  };

  const indexSummary = status.indexingFiles
    ? t("spotlight.index.running")
    : t("spotlight.index.summary", {
        files: numberFormat.format(status.filesIndexed),
        apps: numberFormat.format(status.appsIndexed),
      });

  return (
    <>
      <SectionHeader title={t("spotlight.title")} description={t("spotlight.description")} />

      <SettingsGroup title={t("spotlight.activation")}>
        <SettingRow
          label={t("spotlight.enable")}
          description={t("spotlight.enable.description")}
          control={
            <Toggle label={t("spotlight.enable")} checked={spotlight.enabled} onChange={(enabled) => updateSpotlight({ enabled })} />
          }
        />
        <SettingRow
          label={t("spotlight.shortcut")}
          description={
            status.hotkeyError ? (
              <span className="setting-row__error" role="alert">{status.hotkeyError}</span>
            ) : (
              t("spotlight.shortcut.description")
            )
          }
          disabled={!spotlight.enabled}
          control={
            <HotkeyRecorder
              value={spotlight.hotkey}
              disabled={!spotlight.enabled}
              onChange={(hotkey) => updateSpotlight({ hotkey })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("spotlight.notch")}>
        <SettingRow
          label={t("spotlight.idleNotch")}
          description={t("spotlight.idleNotch.description")}
          disabled={!spotlight.enabled}
          control={
            <Toggle
              label={t("spotlight.idleNotch")}
              checked={spotlight.showIdleNotch}
              disabled={!spotlight.enabled}
              onChange={(showIdleNotch) => updateSpotlight({ showIdleNotch })}
            />
          }
        />
        <SettingRow
          label={t("spotlight.hover")}
          description={t("spotlight.hover.description")}
          control={
            <Toggle
              label={t("spotlight.hover")}
              checked={spotlight.hoverToOpen}
              onChange={(hoverToOpen) => updateSpotlight({ hoverToOpen })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("spotlight.web")}>
        <SettingRow
          label={t("spotlight.engine")}
          description={t("spotlight.engine.description")}
          control={
            <Segmented
              label={t("spotlight.engine")}
              value={spotlight.webSearchEngine}
              options={SEARCH_ENGINES}
              onChange={(webSearchEngine) => updateSpotlight({ webSearchEngine })}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t("spotlight.files")}>
        <SettingRow
          label={t("spotlight.indexFiles")}
          description={t("spotlight.indexFiles.description")}
          control={
            <Toggle
              label={t("spotlight.indexFiles")}
              checked={spotlight.indexFiles}
              onChange={(indexFiles) => updateSpotlight({ indexFiles })}
            />
          }
        />
        <SettingRow
          label={t("spotlight.folders")}
          description={t("spotlight.folders.description")}
          disabled={!spotlight.indexFiles}
        >
          <ul className="folder-list">
            {spotlight.extraFolders.map((folder) => (
              <li key={folder} className="folder-list__item">
                <span className="folder-list__path">{folder}</span>
                <button
                  type="button"
                  className="icon-button"
                  aria-label={t("spotlight.folders.remove", { folder })}
                  onClick={() => updateSpotlight({ extraFolders: spotlight.extraFolders.filter((path) => path !== folder) })}
                >
                  <X size={14} />
                </button>
              </li>
            ))}
          </ul>
          <button type="button" className="button" disabled={!spotlight.indexFiles} onClick={() => void addFolder()}>
            <FolderPlus size={16} />
            {t("spotlight.folders.add")}
          </button>
        </SettingRow>
        <SettingRow
          label={t("spotlight.index")}
          description={indexSummary}
          control={
            <button type="button" className="button" disabled={status.indexingFiles} onClick={controller.rebuildIndex}>
              <ArrowsClockwise size={15} />
              {t("spotlight.index.rebuild")}
            </button>
          }
        />
      </SettingsGroup>
    </>
  );
}
