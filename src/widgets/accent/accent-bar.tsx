import { useRef } from "react";

import type { AccentBarState } from "../../shared/types";

import "./accent-bar.css";

/** Width of one character cell, in CSS pixels (keep in sync with accent-bar.css). */
export const ACCENT_CELL_WIDTH = 40;
/** Padding on each side of the row of cells. */
export const ACCENT_BAR_PADDING = 12;

/** Notch width needed to show `count` characters. */
export function accentBarWidth(count: number): number {
  return count * ACCENT_CELL_WIDTH + ACCENT_BAR_PADDING * 2;
}

/**
 * Quick Accent's characters, drawn inside the notch. The selection is a pill
 * in the accent color that glides from cell to cell. While the bar closes,
 * the last characters stay on screen so they fade out with the notch instead
 * of vanishing first.
 */
export function AccentBar({ state, label }: { state: AccentBarState | null; label: string }) {
  const lastShown = useRef<AccentBarState | null>(null);
  if (state?.visible && state.characters.length > 0) lastShown.current = state;
  const shown = lastShown.current;
  const visible = Boolean(state?.visible);

  return (
    <div className="accent-bar" data-visible={visible} aria-hidden={!visible}>
      {shown && (
        <div className="accent-bar__row" role="listbox" aria-label={label}>
          <span
            className="accent-bar__selection"
            style={{ transform: `translateX(${shown.index * ACCENT_CELL_WIDTH}px)` }}
          />
          {shown.characters.map((character, index) => (
            <span
              key={`${index}-${character}`}
              className="accent-bar__cell"
              role="option"
              aria-selected={index === shown.index}
              style={{ animationDelay: `${index * 18}ms` }}
            >
              {character}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
