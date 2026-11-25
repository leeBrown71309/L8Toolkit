import { ArrowsClockwise, CheckCircle, DownloadSimple, ShieldCheck, WarningCircle } from "@phosphor-icons/react";

import { useI18n } from "../../i18n/i18n-context";
import type { SettingsSnapshot, UpdateState } from "../../shared/types";
import { SectionHeader, SettingRow, SettingsGroup } from "../components/controls";
import type { UpdateController } from "../use-update-state";

interface UpdatesSectionProps {
  snapshot: SettingsSnapshot;
  updates: UpdateController;
}

export function UpdatesSection({ snapshot, updates }: UpdatesSectionProps) {
  const { t } = useI18n();
  const { state } = updates;

  return (
    <>
      <SectionHeader title={t("updates.title")} description={t("updates.description")} />

      <SettingsGroup title={t("updates.status")}>
        <UpdateCard state={state} onCheck={updates.check} onInstall={updates.install} />
      </SettingsGroup>

      <SettingsGroup title={t("updates.about")}>
        <SettingRow label={t("general.version")} description={`L8 Toolkit ${snapshot.status.version}`} />
        <SettingRow label={t("updates.signed")} description={t("updates.signed.description")} />
      </SettingsGroup>
    </>
  );
}

interface UpdateCardProps {
  state: UpdateState;
  onCheck: () => void;
  onInstall: () => void;
}

function UpdateCard({ state, onCheck, onInstall }: UpdateCardProps) {
  const { t, locale } = useI18n();
  const checking = state.status === "checking";

  switch (state.status) {
    case "available":
      return (
        <div className="update-card" data-tone="accent">
          <div className="update-card__head">
            <span className="update-card__icon">
              <DownloadSimple size={18} weight="bold" />
            </span>
            <div className="update-card__text">
              <span className="update-card__title">{t("updates.available", { version: state.version })}</span>
              {state.publishedAtMs !== null && (
                <span className="update-card__meta">
                  {t("updates.publishedOn", { date: formatDate(state.publishedAtMs, locale) })}
                </span>
              )}
            </div>
            <button type="button" className="button button--primary" onClick={onInstall}>
              {t("updates.install")}
            </button>
          </div>
          {state.notes.trim() && <p className="update-card__notes">{state.notes.trim()}</p>}
          <p className="update-card__hint">{t("updates.restartHint")}</p>
        </div>
      );
    case "downloading": {
      const fraction = state.totalBytes ? Math.min(1, state.downloadedBytes / state.totalBytes) : null;
      return (
        <div className="update-card" data-tone="accent">
          <div className="update-card__head">
            <span className="update-card__icon">
              <DownloadSimple size={18} weight="bold" />
            </span>
            <div className="update-card__text">
              <span className="update-card__title">{t("updates.downloading", { version: state.version })}</span>
              <span className="update-card__meta">
                {fraction === null
                  ? t("updates.megabytes", { size: formatMegabytes(state.downloadedBytes, locale) })
                  : t("updates.progress", {
                      percent: String(Math.round(fraction * 100)),
                      size: t("updates.megabytes", { size: formatMegabytes(state.totalBytes ?? 0, locale) }),
                    })}
              </span>
            </div>
          </div>
          <div
            className="update-progress"
            role="progressbar"
            aria-label={t("updates.downloading", { version: state.version })}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={fraction === null ? undefined : Math.round(fraction * 100)}
            data-indeterminate={fraction === null}
          >
            <span className="update-progress__fill" style={{ transform: `scaleX(${fraction ?? 0.3})` }} />
          </div>
        </div>
      );
    }
    case "installing":
      return (
        <div className="update-card" data-tone="accent">
          <div className="update-card__head">
            <span className="update-card__icon">
              <ShieldCheck size={18} weight="bold" />
            </span>
            <div className="update-card__text">
              <span className="update-card__title">{t("updates.installing", { version: state.version })}</span>
              <span className="update-card__meta">{t("updates.restartHint")}</span>
            </div>
          </div>
        </div>
      );
    case "failed":
      return (
        <div className="update-card" data-tone="danger">
          <div className="update-card__head">
            <span className="update-card__icon">
              <WarningCircle size={18} weight="fill" />
            </span>
            <div className="update-card__text">
              <span className="update-card__title">{t("updates.failed")}</span>
              <span className="update-card__meta">{state.message}</span>
            </div>
            <button type="button" className="button" onClick={onCheck}>
              <ArrowsClockwise size={15} />
              {t("updates.retry")}
            </button>
          </div>
        </div>
      );
    default:
      return (
        <div className="update-card">
          <div className="update-card__head">
            <span className="update-card__icon">
              {state.status === "upToDate" ? (
                <CheckCircle size={18} weight="fill" />
              ) : (
                <ArrowsClockwise size={18} className={checking ? "update-card__spinner" : undefined} />
              )}
            </span>
            <div className="update-card__text">
              <span className="update-card__title">
                {checking
                  ? t("updates.checking")
                  : state.status === "upToDate"
                    ? t("updates.upToDate")
                    : t("updates.notChecked")}
              </span>
              <span className="update-card__meta">{t("updates.automatic")}</span>
            </div>
            <button type="button" className="button" onClick={onCheck} disabled={checking}>
              {t("updates.checkNow")}
            </button>
          </div>
        </div>
      );
  }
}

function formatDate(milliseconds: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(milliseconds);
}

/** The number only; the unit is translated ("MB" / "Mo"). */
function formatMegabytes(bytes: number, locale: string): string {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes / 1_000_000);
}
