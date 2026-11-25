import { CheckCircle, MagnifyingGlass, Warning, WarningCircle } from "@phosphor-icons/react";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend, describeError } from "../../shared/backend";
import { Keycap, KeyHintView } from "../../shared/keycap";
import type { SearchResult } from "../../shared/types";
import { ACTION_OPTION_ID_PREFIX, ActionMenu } from "./components/action-menu";
import { CapabilityHints, FrequentGrid } from "./components/idle-suggestions";
import { RESULT_OPTION_ID_PREFIX, ResultList } from "./components/result-list";
import { useActionRunner, type FeedbackTone } from "./hooks/use-action-runner";
import { useFrequentItems, useSearch } from "./hooks/use-search";
import { actionsFor, findActionForKey, shortcutKeycaps } from "./result-actions";

import "./spotlight.css";

const RESULTS_ELEMENT_ID = "spotlight-results";

const FEEDBACK_ICONS = { success: CheckCircle, warning: Warning, error: WarningCircle } satisfies Record<
  FeedbackTone,
  unknown
>;

interface SpotlightPanelProps {
  isOpen: boolean;
  /** This widget's tab is the visible one. */
  isActive: boolean;
  onClose: (restoreFocus: boolean) => void;
  /** The user interacted: a hover "peek" must no longer auto-close. */
  onEngaged: () => void;
}

interface MenuState {
  open: boolean;
  index: number;
}

const CLOSED_MENU: MenuState = { open: false, index: 0 };

function isMenuToggle(event: KeyboardEvent): boolean {
  const ctrlK = event.ctrlKey && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "k";
  const tab = event.key === "Tab" && !event.shiftKey && !event.ctrlKey && !event.altKey;
  return ctrlK || tab;
}

export function SpotlightPanel({ isOpen, isActive, onClose, onEngaged }: SpotlightPanelProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [menu, setMenu] = useState<MenuState>(CLOSED_MENU);
  const [reloadToken, setReloadToken] = useState(0);

  const { sections, error: searchError } = useSearch(query, reloadToken);
  const frequent = useFrequentItems(isOpen, reloadToken);
  const isEmptyQuery = query.trim() === "";
  const items = useMemo(
    () => (isEmptyQuery ? frequent : sections.flatMap((section) => section.items)),
    [isEmptyQuery, frequent, sections],
  );
  const selected: SearchResult | undefined = items[selectedIndex];
  const actions = useMemo(
    () => (selected ? actionsFor(selected, { inFrequent: isEmptyQuery }, t) : []),
    [selected, isEmptyQuery, t],
  );

  const runner = useActionRunner({
    onEngaged,
    onRefresh: useCallback(() => setReloadToken((token) => token + 1), []),
    onSettled: useCallback(() => setMenu(CLOSED_MENU), []),
    t,
  });

  // New results always start on the best match, with the action menu closed.
  useEffect(() => {
    setSelectedIndex(0);
    setMenu(CLOSED_MENU);
  }, [items]);

  useEffect(() => {
    if (!isOpen || !isActive) return;
    const frame = requestAnimationFrame(() => inputRef.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [isOpen, isActive]);

  const openSettings = useCallback(() => {
    backend.openSettings().catch((error: unknown) => runner.showFeedback({ tone: "error", message: describeError(error) }));
  }, [runner]);

  const openActionsFor = (index: number) => {
    setSelectedIndex(index);
    setMenu({ open: true, index: 0 });
    inputRef.current?.focus();
  };

  const moveSelection = (delta: number) => {
    if (items.length === 0) return;
    setSelectedIndex((current) => (current + delta + items.length) % items.length);
  };

  const moveMenu = (delta: number) => {
    setMenu((current) => ({ ...current, index: (current.index + delta + actions.length) % actions.length }));
  };

  const runPrimary = (result: SearchResult) => {
    const primary = actionsFor(result, { inFrequent: isEmptyQuery }, t)[0];
    if (primary) void runner.run(result, primary);
  };

  // Keyboard actions are instant on purpose: no animation on anything the
  // user triggers dozens of times a day.
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    onEngaged();

    if (isMenuToggle(event)) {
      event.preventDefault();
      if (selected) setMenu((current) => (current.open ? CLOSED_MENU : { open: true, index: 0 }));
      return;
    }

    if (menu.open) {
      const plain = !event.ctrlKey && !event.shiftKey && !event.altKey;
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        moveMenu(event.key === "ArrowDown" ? 1 : -1);
        return;
      }
      if (event.key === "Enter" && plain && selected) {
        event.preventDefault();
        const action = actions[menu.index];
        if (action) void runner.run(selected, action);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setMenu(CLOSED_MENU);
        return;
      }
    }

    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp":
        event.preventDefault();
        moveSelection(event.key === "ArrowDown" ? 1 : -1);
        return;
      case "ArrowRight":
      case "ArrowLeft":
        // The frequent row is horizontal; with an empty query the caret has nowhere to go anyway.
        if (isEmptyQuery) {
          event.preventDefault();
          moveSelection(event.key === "ArrowRight" ? 1 : -1);
        }
        return;
      case "Escape":
        event.preventDefault();
        if (query) setQuery("");
        else onClose(true);
        return;
    }

    if (event.ctrlKey && event.key === ",") {
      event.preventDefault();
      openSettings();
      return;
    }

    const action = selected && findActionForKey(actions, event);
    if (selected && action) {
      event.preventDefault();
      void runner.run(selected, action);
    }
  };

  const feedback = runner.feedback ?? (searchError ? { tone: "error" as const, message: searchError } : null);
  const FeedbackIcon = feedback ? FEEDBACK_ICONS[feedback.tone] : null;
  const activeDescendant = menu.open
    ? `${ACTION_OPTION_ID_PREFIX}${menu.index}`
    : !isEmptyQuery && selected
      ? `${RESULT_OPTION_ID_PREFIX}${selectedIndex}`
      : undefined;
  const primaryAction = actions[0];

  return (
    <div className="spotlight">
      <div className="spotlight__search">
        <MagnifyingGlass className="spotlight__search-glyph" size={20} />
        <input
          ref={inputRef}
          className="spotlight__input"
          value={query}
          placeholder={t("search.placeholder")}
          spellCheck={false}
          autoComplete="off"
          role="combobox"
          aria-label={t("search.label")}
          aria-expanded={items.length > 0}
          aria-controls={RESULTS_ELEMENT_ID}
          aria-activedescendant={activeDescendant}
          onChange={(event) => {
            onEngaged();
            setMenu(CLOSED_MENU);
            setQuery(event.target.value);
          }}
          onKeyDown={handleKeyDown}
        />
        <Keycap>esc</Keycap>
      </div>

      <div id={RESULTS_ELEMENT_ID} className="spotlight__results">
        {menu.open && selected ? (
          <ActionMenu
            result={selected}
            actions={actions}
            selectedIndex={menu.index}
            confirmingActionId={runner.confirmingActionId(selected.id)}
            onHover={(index) => setMenu({ open: true, index })}
            onRun={(action) => void runner.run(selected, action)}
          />
        ) : isEmptyQuery ? (
          frequent.length > 0 ? (
            <FrequentGrid
              items={frequent}
              selectedIndex={selectedIndex}
              onHover={setSelectedIndex}
              onExecute={runPrimary}
              onOpenActions={openActionsFor}
            />
          ) : (
            <CapabilityHints />
          )
        ) : (
          <ResultList
            sections={sections}
            selectedIndex={selectedIndex}
            copiedResultId={runner.copiedResultId}
            onHover={setSelectedIndex}
            onExecute={runPrimary}
            onOpenActions={openActionsFor}
          />
        )}
      </div>

      <footer className="spotlight__footer">
        {feedback && FeedbackIcon ? (
          <span className="spotlight__feedback" data-tone={feedback.tone} role="status">
            <FeedbackIcon size={14} weight="fill" />
            {feedback.message}
          </span>
        ) : (
          <div className="spotlight__hints">
            {menu.open ? (
              <KeyHintView hint={{ keys: ["↵"], label: t("search.hint.run") }} />
            ) : (
              primaryAction?.shortcut && (
                <KeyHintView hint={{ keys: shortcutKeycaps(primaryAction.shortcut, t), label: primaryAction.label }} />
              )
            )}
          </div>
        )}
        <div className="spotlight__hints">
          {menu.open ? (
            <KeyHintView hint={{ keys: ["esc"], label: t("search.hint.back") }} />
          ) : (
            actions.length > 1 && <KeyHintView hint={{ keys: ["Ctrl", "K"], label: t("search.hint.actions") }} />
          )}
        </div>
      </footer>
    </div>
  );
}
