import { Equals, FolderSimple, Globe, PushPin } from "@phosphor-icons/react";

import { useI18n } from "../../../i18n/i18n-context";
import type { MessageKey } from "../../../i18n/messages/en";
import type { SearchResult } from "../../../shared/types";
import { ResultIcon } from "./result-icon";

interface FrequentGridProps {
  items: SearchResult[];
  selectedIndex: number;
  onHover: (index: number) => void;
  onExecute: (result: SearchResult) => void;
  onOpenActions: (index: number) => void;
}

/** Pins and what you launch most, one keystroke away before typing anything. */
export function FrequentGrid({ items, selectedIndex, onHover, onExecute, onOpenActions }: FrequentGridProps) {
  const { t } = useI18n();
  return (
    <div className="frequent">
      <div className="result-section__label">{t("search.frequent")}</div>
      <div className="frequent__grid" role="listbox" aria-label={t("search.frequent")} aria-orientation="horizontal">
        {items.map((item, index) => (
          <button
            key={item.id}
            type="button"
            role="option"
            aria-selected={index === selectedIndex}
            tabIndex={-1}
            className="frequent__tile"
            data-selected={index === selectedIndex}
            onPointerMove={() => {
              if (index !== selectedIndex) onHover(index);
            }}
            onClick={() => onExecute(item)}
            onContextMenu={(event) => {
              event.preventDefault();
              onOpenActions(index);
            }}
          >
            <span className="frequent__icon">
              <ResultIcon result={item} size={40} />
              {item.pinned && (
                <span className="frequent__pin" aria-label={t("search.pinned")}>
                  <PushPin size={9} weight="fill" />
                </span>
              )}
            </span>
            <span className="frequent__name">{item.title}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

const CAPABILITIES: { icon: typeof FolderSimple; labelKey: MessageKey }[] = [
  { icon: FolderSimple, labelKey: "search.capability.files" },
  { icon: Equals, labelKey: "search.capability.math" },
  { icon: Globe, labelKey: "search.capability.web" },
];

/** First run: nothing launched yet, so explain what the field understands. */
export function CapabilityHints() {
  const { t } = useI18n();
  return (
    <ul className="capabilities" aria-label={t("search.capabilities")}>
      {CAPABILITIES.map(({ icon: Glyph, labelKey }) => (
        <li key={labelKey} className="capabilities__item">
          <Glyph size={15} />
          {t(labelKey)}
        </li>
      ))}
    </ul>
  );
}
