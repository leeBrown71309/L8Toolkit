/**
 * Keyboard shortcut helpers. Accelerators use the format understood by the
 * Rust `global-hotkey` crate: "Ctrl+Alt+Space", with "Super" for the Windows key.
 */

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta", "OS", "AltGraph"]);

const DISPLAY_NAMES: Record<string, string> = {
  Super: "Win",
  Control: "Ctrl",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Backslash: "\\",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

/** Splits an accelerator into keycap labels for display. */
export function acceleratorToKeycaps(accelerator: string): string[] {
  return accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => DISPLAY_NAMES[part] ?? part);
}

function keyTokenFromCode(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (code === "Space" || code === "Enter" || code === "Tab" || code === "Backspace") return code;
  if (code in DISPLAY_NAMES) return code;
  return null;
}

export type RecordedShortcut =
  | { status: "incomplete" }
  | { status: "invalid"; reason: "unsupportedKey" | "needsModifier" }
  | { status: "complete"; accelerator: string };

/**
 * Builds an accelerator from a key press. A shortcut needs at least one
 * modifier unless it is a function key, otherwise it would swallow typing.
 */
export function recordShortcut(event: KeyboardEvent): RecordedShortcut {
  if (MODIFIER_KEYS.has(event.key)) return { status: "incomplete" };

  const key = keyTokenFromCode(event.code);
  if (!key) return { status: "invalid", reason: "unsupportedKey" };

  const modifiers = [
    event.ctrlKey && "Ctrl",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Super",
  ].filter((modifier): modifier is string => Boolean(modifier));

  const isFunctionKey = /^F\d{1,2}$/.test(key);
  if (modifiers.length === 0 && !isFunctionKey) {
    return { status: "invalid", reason: "needsModifier" };
  }
  return { status: "complete", accelerator: [...modifiers, key].join("+") };
}
