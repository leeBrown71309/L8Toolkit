import type { MessageKey } from "../../i18n/messages/en";

/** A key is either a symbol shown as is ("←") or a translated name ("Suppr"). */
type ShortcutKey = { symbol: string } | { name: MessageKey };

export interface RecentsShortcut {
  keys: ShortcutKey[];
  /** Full sentence for Settings › Recents. */
  descriptionKey: MessageKey;
}

/**
 * The Recents keyboard shortcuts, as handled by `RecentsPanel`. Listed in
 * Settings › Recents rather than in the notch, which stays uncluttered.
 */
export const RECENTS_SHORTCUTS: RecentsShortcut[] = [
  { keys: [{ symbol: "←" }, { symbol: "→" }], descriptionKey: "recentsSettings.shortcut.select" },
  { keys: [{ name: "keys.home" }, { name: "keys.end" }], descriptionKey: "recentsSettings.shortcut.ends" },
  { keys: [{ symbol: "↵" }], descriptionKey: "recentsSettings.shortcut.copy" },
  { keys: [{ symbol: "Ctrl" }, { symbol: "↵" }], descriptionKey: "recentsSettings.shortcut.open" },
  { keys: [{ name: "keys.delete" }], descriptionKey: "recentsSettings.shortcut.remove" },
];

export function shortcutKeyLabel(key: ShortcutKey, t: (key: MessageKey) => string): string {
  return "symbol" in key ? key.symbol : t(key.name);
}
