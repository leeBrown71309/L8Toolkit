import { useEffect, useRef } from "react";

import type { SearchResult } from "../../../shared/types";
import { useI18n } from "../../../i18n/i18n-context";
import { shortcutKeycaps, type ActionId, type ResultAction } from "../result-actions";
import { Keycap } from "../../../shared/keycap";
import { ResultIcon } from "./result-icon";

export const ACTION_OPTION_ID_PREFIX = "spotlight-action-";

interface ActionMenuProps {
  result: SearchResult;
  actions: ResultAction[];
  selectedIndex: number;
  /** Destructive action waiting for its confirming second press. */
  confirmingActionId: ActionId | null;
  onHover: (index: number) => void;
  onRun: (action: ResultAction) => void;
}

/**
 * Everything you can do with the selected result. It replaces the result
 * list in place, so the notch simply grows or shrinks to fit it.
 */
export function ActionMenu({ result, actions, selectedIndex, confirmingActionId, onHover, onRun }: ActionMenuProps) {
  const { t } = useI18n();
  return (
    <div className="action-menu" role="menu" aria-label={t("actions.menuLabel", { title: result.title })}>
      <div className="action-menu__header">
        <ResultIcon result={result} size={20} />
        <span className="action-menu__title">{result.title}</span>
        <span className="action-menu__count">{t("actions.count", { count: actions.length })}</span>
      </div>
      {actions.map((action, index) => (
        <ActionRow
          key={action.id}
          index={index}
          action={action}
          selected={index === selectedIndex}
          confirming={action.id === confirmingActionId}
          onHover={onHover}
          onRun={onRun}
        />
      ))}
    </div>
  );
}

interface ActionRowProps {
  index: number;
  action: ResultAction;
  selected: boolean;
  confirming: boolean;
  onHover: (index: number) => void;
  onRun: (action: ResultAction) => void;
}

function ActionRow({ index, action, selected, confirming, onHover, onRun }: ActionRowProps) {
  const { t } = useI18n();
  const rowRef = useRef<HTMLDivElement>(null);
  const Glyph = action.icon;

  useEffect(() => {
    if (selected) rowRef.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  return (
    <div
      ref={rowRef}
      id={`${ACTION_OPTION_ID_PREFIX}${index}`}
      role="menuitem"
      className="action-row"
      data-selected={selected}
      data-destructive={Boolean(action.destructive)}
      data-confirming={confirming}
      onPointerMove={() => {
        if (!selected) onHover(index);
      }}
      onClick={() => onRun(action)}
    >
      <Glyph size={16} className="action-row__icon" />
      <span className="action-row__label">{confirming ? t("actions.confirmAgain") : action.label}</span>
      {action.shortcut && (
        <span className="action-row__keys">
          {shortcutKeycaps(action.shortcut, t).map((key) => (
            <Keycap key={key}>{key}</Keycap>
          ))}
        </span>
      )}
    </div>
  );
}
