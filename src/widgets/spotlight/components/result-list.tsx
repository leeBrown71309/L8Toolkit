import { CheckCircle, PushPin } from "@phosphor-icons/react";
import { useEffect, useRef } from "react";

import type { SearchResult, SearchSection, SectionId } from "../../../shared/types";
import { actionsFor } from "../result-actions";
import { useI18n } from "../../../i18n/i18n-context";
import { SECTION_LABEL_KEYS } from "../result-presentation";
import { HighlightedText } from "./highlighted-text";
import { Keycap } from "../../../shared/keycap";
import { ResultIcon } from "./result-icon";

export const RESULT_OPTION_ID_PREFIX = "spotlight-option-";

interface ResultListProps {
  sections: SearchSection[];
  selectedIndex: number;
  copiedResultId: string | null;
  onHover: (index: number) => void;
  onExecute: (result: SearchResult) => void;
  onOpenActions: (index: number) => void;
}

/** Flat keyboard index across sections, rendered with section labels. */
export function ResultList({ sections, selectedIndex, copiedResultId, onHover, onExecute, onOpenActions }: ResultListProps) {
  const { t } = useI18n();
  let flatIndex = 0;
  return (
    <div className="result-list" role="listbox" aria-label={t("search.results")}>
      {sections.map((section) => (
        <div key={section.id} role="group" aria-label={t(SECTION_LABEL_KEYS[section.id])} className="result-section">
          <div className="result-section__label" aria-hidden="true">
            {t(SECTION_LABEL_KEYS[section.id])}
          </div>
          {section.items.map((result) => {
            const index = flatIndex++;
            return (
              <ResultRow
                key={result.id}
                index={index}
                result={result}
                sectionId={section.id}
                selected={index === selectedIndex}
                copied={copiedResultId === result.id}
                onHover={onHover}
                onExecute={onExecute}
                onOpenActions={onOpenActions}
              />
            );
          })}
        </div>
      ))}
    </div>
  );
}

interface ResultRowProps {
  index: number;
  result: SearchResult;
  sectionId: SectionId;
  selected: boolean;
  copied: boolean;
  onHover: (index: number) => void;
  onExecute: (result: SearchResult) => void;
  onOpenActions: (index: number) => void;
}

function rowVariant(sectionId: SectionId): "hero" | "calculator" | "default" {
  if (sectionId === "calculator") return "calculator";
  if (sectionId === "topHit") return "hero";
  return "default";
}

function ResultRow({ index, result, sectionId, selected, copied, onHover, onExecute, onOpenActions }: ResultRowProps) {
  const { t } = useI18n();
  const rowRef = useRef<HTMLDivElement>(null);
  const variant = rowVariant(sectionId);

  useEffect(() => {
    if (selected) rowRef.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const isPath = result.kind === "file" || result.kind === "folder";

  return (
    <div
      ref={rowRef}
      id={`${RESULT_OPTION_ID_PREFIX}${index}`}
      role="option"
      aria-selected={selected}
      className={`result-row result-row--${variant}`}
      data-selected={selected}
      onPointerMove={() => {
        if (!selected) onHover(index);
      }}
      onClick={() => onExecute(result)}
      onContextMenu={(event) => {
        event.preventDefault();
        onOpenActions(index);
      }}
    >
      {variant !== "calculator" && <ResultIcon result={result} size={variant === "hero" ? 36 : 26} />}

      <div className="result-row__text">
        {variant === "calculator" && <span className="result-row__expression">{result.subtitle}</span>}
        <span className="result-row__title">
          <HighlightedText text={result.title} indices={result.matchIndices} />
          {result.pinned && <PushPin className="result-row__pin" size={11} weight="fill" aria-label={t("search.pinned")} />}
        </span>
        {variant !== "calculator" && (
          <span className={`result-row__subtitle${isPath ? " result-row__subtitle--path" : ""}`}>
            {result.subtitle}
          </span>
        )}
      </div>

      {copied ? (
        <span className="result-row__hint result-row__hint--done">
          <CheckCircle size={14} weight="fill" />
          {t("search.copied")}
        </span>
      ) : (
        selected && (
          <span className="result-row__hint">
            {actionsFor(result, { inFrequent: false }, t)[0]?.label}
            <Keycap>↵</Keycap>
          </span>
        )
      )}
    </div>
  );
}
