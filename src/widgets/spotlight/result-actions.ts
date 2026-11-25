import {
  ArrowSquareOut,
  ClipboardText,
  Copy,
  Files,
  FolderOpen,
  Info,
  type Icon,
  Link,
  PushPin,
  PushPinSlash,
  ShieldCheck,
  SquaresFour,
  Terminal,
  Trash,
  XCircle,
  AppWindow,
} from "@phosphor-icons/react";

import type { Translate } from "../../i18n/i18n-context";
import type { ExecuteAction, SearchResult } from "../../shared/types";

/**
 * Every action a result can offer. Most run in Rust; the clipboard-only ones
 * (copy path, copy link, copy result) run in the web layer.
 */
export type ActionId =
  | Exclude<ExecuteAction, "pasteClipboard">
  | "copyPath"
  | "copyLink"
  | "copyResult"
  | "pasteResult";

export interface KeyShortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export interface ResultAction {
  id: ActionId;
  label: string;
  icon: Icon;
  shortcut?: KeyShortcut;
  /** Needs a second press within a few seconds. */
  destructive?: boolean;
}

export interface ActionContext {
  /** The result is shown in the Frequent row (empty query). */
  inFrequent: boolean;
}

// Shortcuts avoid Ctrl+Alt, which is AltGr on many European keyboards.
const ENTER: KeyShortcut = { key: "Enter" };
const CTRL_ENTER: KeyShortcut = { key: "Enter", ctrl: true };
const CTRL_SHIFT_ENTER: KeyShortcut = { key: "Enter", ctrl: true, shift: true };
const ALT_ENTER: KeyShortcut = { key: "Enter", alt: true };
const CTRL_SHIFT_C: KeyShortcut = { key: "c", ctrl: true, shift: true };
const CTRL_O: KeyShortcut = { key: "o", ctrl: true };
const CTRL_T: KeyShortcut = { key: "t", ctrl: true };
const CTRL_P: KeyShortcut = { key: "p", ctrl: true };
const CTRL_SHIFT_DELETE: KeyShortcut = { key: "Delete", ctrl: true, shift: true };

const EXECUTABLE_EXTENSIONS = [".exe", ".bat", ".cmd", ".msi", ".lnk"];

function isExecutable(path: string): boolean {
  const lowercase = path.toLowerCase();
  return EXECUTABLE_EXTENSIONS.some((extension) => lowercase.endsWith(extension));
}

function pinAction(result: SearchResult, t: Translate): ResultAction {
  return result.pinned
    ? { id: "unpin", label: t("action.unpin"), icon: PushPinSlash, shortcut: CTRL_P }
    : { id: "pin", label: t("action.pin"), icon: PushPin, shortcut: CTRL_P };
}

function compact(actions: (ResultAction | false | undefined)[]): ResultAction[] {
  return actions.filter((action): action is ResultAction => Boolean(action));
}

/** Ordered list; the first action is what Enter does. */
export function actionsFor(result: SearchResult, context: ActionContext, t: Translate): ResultAction[] {
  const onDisk = Boolean(result.location);
  const forget = context.inFrequent && { id: "forget" as const, label: t("action.forget"), icon: XCircle };

  switch (result.kind) {
    case "app":
      return compact([
        { id: "open", label: t("action.open"), icon: AppWindow, shortcut: ENTER },
        onDisk && { id: "runAsAdmin", label: t("action.runAsAdmin"), icon: ShieldCheck, shortcut: CTRL_SHIFT_ENTER },
        onDisk && { id: "reveal", label: t("action.reveal"), icon: FolderOpen, shortcut: CTRL_ENTER },
        onDisk && { id: "copyPath", label: t("action.copyPath"), icon: ClipboardText, shortcut: CTRL_SHIFT_C },
        onDisk && { id: "properties", label: t("action.properties"), icon: Info, shortcut: ALT_ENTER },
        pinAction(result, t),
        forget,
        { id: "manageApps", label: t("action.manageApps"), icon: SquaresFour },
      ]);
    case "file":
      return compact([
        { id: "open", label: t("action.open"), icon: ArrowSquareOut, shortcut: ENTER },
        { id: "openWith", label: t("action.openWith"), icon: SquaresFour, shortcut: CTRL_O },
        isExecutable(result.target) && {
          id: "runAsAdmin",
          label: t("action.runAsAdmin"),
          icon: ShieldCheck,
          shortcut: CTRL_SHIFT_ENTER,
        },
        { id: "reveal", label: t("action.reveal"), icon: FolderOpen, shortcut: CTRL_ENTER },
        { id: "copyPath", label: t("action.copyPath"), icon: ClipboardText, shortcut: CTRL_SHIFT_C },
        { id: "copyFile", label: t("action.copyFile"), icon: Files },
        { id: "openInTerminal", label: t("action.openFolderInTerminal"), icon: Terminal, shortcut: CTRL_T },
        { id: "properties", label: t("action.properties"), icon: Info, shortcut: ALT_ENTER },
        pinAction(result, t),
        forget,
        { id: "moveToRecycleBin", label: t("action.moveToRecycleBin"), icon: Trash, shortcut: CTRL_SHIFT_DELETE, destructive: true },
      ]);
    case "folder":
      return compact([
        { id: "open", label: t("action.open"), icon: FolderOpen, shortcut: ENTER },
        { id: "openInTerminal", label: t("action.openInTerminal"), icon: Terminal, shortcut: CTRL_T },
        { id: "reveal", label: t("action.revealParent"), icon: ArrowSquareOut, shortcut: CTRL_ENTER },
        { id: "copyPath", label: t("action.copyPath"), icon: ClipboardText, shortcut: CTRL_SHIFT_C },
        { id: "copyFile", label: t("action.copyFolder"), icon: Files },
        { id: "properties", label: t("action.properties"), icon: Info, shortcut: ALT_ENTER },
        pinAction(result, t),
        forget,
        { id: "moveToRecycleBin", label: t("action.moveToRecycleBin"), icon: Trash, shortcut: CTRL_SHIFT_DELETE, destructive: true },
      ]);
    case "calculator":
      return [
        { id: "copyResult", label: t("action.copyResult"), icon: Copy, shortcut: ENTER },
        { id: "pasteResult", label: t("action.pasteResult"), icon: ClipboardText, shortcut: CTRL_ENTER },
      ];
    case "web":
      return [
        { id: "open", label: result.subtitle, icon: ArrowSquareOut, shortcut: ENTER },
        { id: "copyLink", label: t("action.copySearchLink"), icon: Link, shortcut: CTRL_SHIFT_C },
      ];
    case "url":
      return [
        { id: "open", label: t("action.openInBrowser"), icon: ArrowSquareOut, shortcut: ENTER },
        { id: "copyLink", label: t("action.copyLink"), icon: Link, shortcut: CTRL_SHIFT_C },
      ];
    case "command":
      return [{ id: "open", label: t("action.run"), icon: ArrowSquareOut, shortcut: ENTER }];
  }
}

interface KeyState {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

export function matchesShortcut(event: KeyState, shortcut: KeyShortcut): boolean {
  return (
    event.key.toLowerCase() === shortcut.key.toLowerCase() &&
    event.ctrlKey === Boolean(shortcut.ctrl) &&
    event.shiftKey === Boolean(shortcut.shift) &&
    event.altKey === Boolean(shortcut.alt)
  );
}

export function findActionForKey(actions: ResultAction[], event: KeyState): ResultAction | undefined {
  return actions.find((action) => action.shortcut && matchesShortcut(event, action.shortcut));
}

/** Keycap labels follow the keyboard's language ("Maj", "Suppr" in French). */
export function shortcutKeycaps(shortcut: KeyShortcut, t: Translate): string[] {
  const keyLabels: Record<string, string> = { enter: "↵", delete: t("keys.delete"), backspace: "⌫" };
  const key = keyLabels[shortcut.key.toLowerCase()] ?? shortcut.key.toUpperCase();
  return [shortcut.ctrl && "Ctrl", shortcut.shift && t("keys.shift"), shortcut.alt && "Alt", key].filter(
    (part): part is string => Boolean(part),
  );
}

/** Actions the Rust side executes, as opposed to clipboard-only ones. */
export function toExecuteAction(id: ActionId): ExecuteAction | null {
  switch (id) {
    case "copyPath":
    case "copyLink":
    case "copyResult":
      return null;
    case "pasteResult":
      return "pasteClipboard";
    default:
      return id;
  }
}
