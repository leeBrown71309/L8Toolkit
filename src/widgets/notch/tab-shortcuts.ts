import type { WidgetTab } from "../../shared/types";

/**
 * Widget order in the notch. Each widget keeps its number (Alt+1 to Alt+4)
 * even when another one is turned off, so the hints in Settings stay true.
 */
export const TAB_ORDER: WidgetTab[] = ["search", "music", "timer", "recents"];

/**
 * The widget an Alt+digit press asks for, read from the physical key
 * (`KeyboardEvent.code`): on AZERTY the "3" key types `"`, so `event.key`
 * would never be a digit without Shift.
 */
export function tabForKeyCode(code: string): WidgetTab | undefined {
  const match = /^(?:Digit|Numpad)([1-9])$/.exec(code);
  if (!match) return undefined;
  return TAB_ORDER[Number(match[1]) - 1];
}

/** The digit that goes with a widget, for hints: Alt+3 for the timer. */
export function tabNumber(tab: WidgetTab): number {
  return TAB_ORDER.indexOf(tab) + 1;
}
