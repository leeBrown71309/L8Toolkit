import { ClockCounterClockwise, GearSix, type Icon, MagnifyingGlass, MusicNotesSimple, Timer } from "@phosphor-icons/react";
import { useLayoutEffect, useRef, useState } from "react";

import { useI18n } from "../../i18n/i18n-context";
import type { MessageKey } from "../../i18n/messages/en";
import type { WidgetTab } from "../../shared/types";
import { Equalizer } from "../music/equalizer";

import "./widget-header.css";

const TAB_META: Record<WidgetTab, { labelKey: MessageKey; icon: Icon }> = {
  search: { labelKey: "widget.search", icon: MagnifyingGlass },
  music: { labelKey: "widget.music", icon: MusicNotesSimple },
  timer: { labelKey: "widget.timer", icon: Timer },
  recents: { labelKey: "widget.recents", icon: ClockCounterClockwise },
};

interface IndicatorBox {
  left: number;
  width: number;
}

interface WidgetHeaderProps {
  tabs: WidgetTab[];
  active: WidgetTab;
  /** Media playing: the Music tab shows a live equalizer. */
  musicPlaying: boolean;
  /** A timer counts down: its tab icon takes the accent color. */
  timerRunning: boolean;
  onSelect: (tab: WidgetTab) => void;
  onOpenSettings: () => void;
}

/**
 * Widget switcher living in the notch's "camera" strip. The active pill
 * slides between tabs, so switching reads as one motion, not two states.
 */
export function WidgetHeader({ tabs, active, musicPlaying, timerRunning, onSelect, onOpenSettings }: WidgetHeaderProps) {
  const { t } = useI18n();
  const tabRefs = useRef(new Map<WidgetTab, HTMLButtonElement>());
  const [indicator, setIndicator] = useState<IndicatorBox | null>(null);
  const [animated, setAnimated] = useState(false);

  useLayoutEffect(() => {
    const element = tabRefs.current.get(active);
    if (!element) return;
    setIndicator({ left: element.offsetLeft, width: element.offsetWidth });
  }, [active, tabs, musicPlaying]);

  // The first placement must not slide in from the left edge.
  useLayoutEffect(() => {
    if (!indicator || animated) return;
    const frame = requestAnimationFrame(() => setAnimated(true));
    return () => cancelAnimationFrame(frame);
  }, [indicator, animated]);

  return (
    <header className="widget-header">
      <nav className="widget-tabs" aria-label={t("widget.tabs")}>
        {indicator && (
          <span
            className="widget-tabs__indicator"
            data-animated={animated}
            style={{ width: indicator.width, transform: `translateX(${indicator.left}px)` }}
            aria-hidden="true"
          />
        )}
        {tabs.map((tab) => {
          const { labelKey, icon: Glyph } = TAB_META[tab];
          const label = t(labelKey);
          const isActive = tab === active;
          const showEqualizer = tab === "music" && musicPlaying;
          return (
            <button
              key={tab}
              ref={(element) => {
                if (element) tabRefs.current.set(tab, element);
                else tabRefs.current.delete(tab);
              }}
              type="button"
              className="widget-tab"
              data-active={isActive}
              data-running={tab === "timer" && timerRunning}
              aria-current={isActive ? "page" : undefined}
              title={t("widget.switchHint", { widget: label })}
              onClick={() => onSelect(tab)}
            >
              {showEqualizer ? <Equalizer playing className="widget-tab__equalizer" /> : <Glyph size={13} weight="bold" />}
              {label}
            </button>
          );
        })}
      </nav>
      <button type="button" className="icon-button" aria-label={t("widget.openSettings")} title={`${t("widget.openSettings")} (Ctrl+,)`} onClick={onOpenSettings}>
        <GearSix size={16} />
      </button>
    </header>
  );
}
