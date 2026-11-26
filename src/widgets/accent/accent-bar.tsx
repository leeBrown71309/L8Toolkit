import { useRef } from "react";

import type { AccentBarState } from "../../shared/types";

import "./accent-bar.css";

/** Width of one character cell, in CSS pixels (keep in sync with accent-bar.css). */
export const ACCENT_CELL_WIDTH = 40;
/** Padding on each side of the row of cells. */
export const ACCENT_BAR_PADDING = 12;
/** Cells shown at once; longer rows (Vietnamese, all languages) scroll. */
export const ACCENT_VISIBLE_CELLS = 15;

/** Notch width needed to show `count` characters. */
export function accentBarWidth(count: number): number {
  return Math.min(count, ACCENT_VISIBLE_CELLS) * ACCENT_CELL_WIDTH + ACCENT_BAR_PADDING * 2;
}

/** First visible cell: keeps the selection near the middle of a long row. */
export function accentScrollOffset(count: number, index: number): number {
  const overflow = count - ACCENT_VISIBLE_CELLS;
  if (overflow <= 0) return 0;
  return Math.min(overflow, Math.max(0, index - Math.floor(ACCENT_VISIBLE_CELLS / 2)));
}

/**
 * Quick Accent's characters, drawn inside the notch. The selection is a pill
 * in the accent color that glides from cell to cell; a long row slides under
 * it so the selection stays in view. While the bar closes, the last
 * characters stay on screen so they fade out with the notch.
 */
export function AccentBar({ state, label }: { state: AccentBarState | null; label: string }) {
  const lastShown = useRef<AccentBarState | null>(null);
  if (state?.visible && state.characters.length > 0) lastShown.current = state;
  const shown = lastShown.current;
  const visible = Boolean(state?.visible);

  if (!shown) return <div className="accent-bar" data-visible={false} aria-hidden="true" />;

  const count = shown.characters.length;
  const offset = accentScrollOffset(count, shown.index);
  const scrollable = count > ACCENT_VISIBLE_CELLS;

  return (
    <div className="accent-bar" data-visible={visible} aria-hidden={!visible}>
      <div
        className="accent-bar__viewport"
        data-more-before={scrollable && offset > 0}
        data-more-after={scrollable && offset < count - ACCENT_VISIBLE_CELLS}
        style={{ width: Math.min(count, ACCENT_VISIBLE_CELLS) * ACCENT_CELL_WIDTH }}
      >
        <div
          className="accent-bar__row"
          role="listbox"
          aria-label={label}
          style={{ transform: `translateX(${-offset * ACCENT_CELL_WIDTH}px)` }}
        >
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
              style={{ animationDelay: `${Math.min(index, ACCENT_VISIBLE_CELLS) * 18}ms` }}
            >
              {character}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}
