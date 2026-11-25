import {
  AppWindow,
  ArrowsClockwise,
  Equals,
  File,
  Folder,
  GearSix,
  Globe,
  type Icon,
  LockSimple,
  MagnifyingGlass,
  Power,
  SlidersHorizontal,
  Trash,
} from "@phosphor-icons/react";

import type { MessageKey } from "../../i18n/messages/en";
import type { ResultKind, SearchResult, SectionId } from "../../shared/types";

export const SECTION_LABEL_KEYS: Record<SectionId, MessageKey> = {
  calculator: "section.calculator",
  topHit: "section.topHit",
  applications: "section.applications",
  commands: "section.commands",
  folders: "section.folders",
  files: "section.files",
  web: "section.web",
};

/** Ids mirror `search/system_actions.rs`. */
const COMMAND_GLYPHS: Record<string, Icon> = {
  "l8.settings": GearSix,
  "l8.reindex": ArrowsClockwise,
  "l8.quit": Power,
  "system.lock": LockSimple,
  "system.settings": SlidersHorizontal,
  "system.recycle-bin": Trash,
};

const KIND_GLYPHS: Record<ResultKind, Icon> = {
  app: AppWindow,
  file: File,
  folder: Folder,
  calculator: Equals,
  web: MagnifyingGlass,
  url: Globe,
  command: GearSix,
};

export function glyphFor(result: SearchResult): Icon {
  if (result.kind === "command") return COMMAND_GLYPHS[result.target] ?? GearSix;
  return KIND_GLYPHS[result.kind];
}
