import { Camera } from "@phosphor-icons/react";

import { useI18n } from "../../i18n/i18n-context";
import type { RecentView } from "../../shared/types";

import "./screenshot-live.css";

/**
 * A screenshot was just taken: the notch peeks with its thumbnail, the way
 * phones show a capture in the corner. It stays in Recents afterwards.
 */
export function ScreenshotLive({ shot, visible }: { shot: RecentView; visible: boolean }) {
  const { t } = useI18n();
  return (
    <div className="shot-live" data-visible={visible} aria-hidden={!visible}>
      <span className="shot-live__thumb">
        {shot.thumbnail ? <img src={shot.thumbnail} alt="" draggable={false} /> : <Camera size={18} />}
      </span>
      <span className="shot-live__text">
        <span className="shot-live__title">{t("recents.screenshot.title")}</span>
        <span className="shot-live__subtitle">{t("recents.screenshot.text")}</span>
      </span>
    </div>
  );
}
