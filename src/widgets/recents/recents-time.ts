import type { MessageKey } from "../../i18n/messages/en";

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** "now", "5 min", "3 h", "2 d": a message key and its count. */
export function relativeAge(createdAtMs: number, now: number = Date.now()): { key: MessageKey; count: number } {
  const age = Math.max(0, now - createdAtMs);
  if (age < MINUTE_MS) return { key: "recents.time.now", count: 0 };
  if (age < HOUR_MS) return { key: "recents.time.minutes", count: Math.floor(age / MINUTE_MS) };
  if (age < DAY_MS) return { key: "recents.time.hours", count: Math.floor(age / HOUR_MS) };
  return { key: "recents.time.days", count: Math.floor(age / DAY_MS) };
}

/** Pixels the pointer must travel with the button held before a click becomes a drag. */
export const DRAG_THRESHOLD_PX = 5;

export function isDragGesture(startX: number, startY: number, x: number, y: number): boolean {
  return Math.hypot(x - startX, y - startY) >= DRAG_THRESHOLD_PX;
}
