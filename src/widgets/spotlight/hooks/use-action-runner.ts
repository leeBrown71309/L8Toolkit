import { useCallback, useEffect, useState } from "react";

import type { Translate } from "../../../i18n/i18n-context";
import { backend, describeError } from "../../../shared/backend";
import type { SearchResult } from "../../../shared/types";
import { toExecuteAction, type ActionId, type ResultAction } from "../result-actions";

const FEEDBACK_DURATION_MS = 3200;
const COPIED_BADGE_DURATION_MS = 1400;
/** Window in which the second press of a destructive action counts. */
const CONFIRM_WINDOW_MS = 3200;

export type FeedbackTone = "success" | "warning" | "error";

export interface Feedback {
  tone: FeedbackTone;
  message: string;
}

interface PendingConfirmation {
  resultId: string;
  actionId: ActionId;
}

interface ActionRunnerOptions {
  onEngaged: () => void;
  /** Results changed on disk or in the history (pin, delete). */
  onRefresh: () => void;
  /** An action finished without closing the notch. */
  onSettled: () => void;
  t: Translate;
}

export interface ActionRunner {
  run: (result: SearchResult, action: ResultAction) => Promise<void>;
  feedback: Feedback | null;
  showFeedback: (feedback: Feedback) => void;
  copiedResultId: string | null;
  confirmingActionId: (resultId: string | undefined) => ActionId | null;
}

function useTransient<T>(durationMs: number): [T | null, (value: T | null) => void] {
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    if (value === null) return;
    const timer = window.setTimeout(() => setValue(null), durationMs);
    return () => window.clearTimeout(timer);
  }, [value, durationMs]);
  return [value, setValue];
}

async function writeClipboard(text: string, t: Translate): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    throw new Error(t("feedback.clipboardFailed", { reason: describeError(error) }));
  }
}

export function useActionRunner({ onEngaged, onRefresh, onSettled, t }: ActionRunnerOptions): ActionRunner {
  const [feedback, setFeedback] = useTransient<Feedback>(FEEDBACK_DURATION_MS);
  const [copiedResultId, setCopiedResultId] = useTransient<string>(COPIED_BADGE_DURATION_MS);
  const [pending, setPending] = useTransient<PendingConfirmation>(CONFIRM_WINDOW_MS);

  const run = useCallback(
    async (result: SearchResult, action: ResultAction) => {
      onEngaged();
      if (action.destructive) {
        const confirmed = pending?.resultId === result.id && pending.actionId === action.id;
        if (!confirmed) {
          setPending({ resultId: result.id, actionId: action.id });
          setFeedback({ tone: "warning", message: t("feedback.confirmRecycle", { title: result.title }) });
          return;
        }
        setPending(null);
      }

      try {
        switch (action.id) {
          case "copyPath":
            await writeClipboard(result.location ?? result.target, t);
            setFeedback({ tone: "success", message: t("feedback.pathCopied") });
            break;
          case "copyLink":
            await writeClipboard(result.target, t);
            setFeedback({ tone: "success", message: t("feedback.linkCopied") });
            break;
          case "copyResult":
            await writeClipboard(result.target, t);
            setCopiedResultId(result.id);
            break;
          case "pasteResult":
            await writeClipboard(result.target, t);
            await backend.executeResult(result, "pasteClipboard");
            break;
          default: {
            const executeAction = toExecuteAction(action.id);
            if (!executeAction) return;
            const outcome = await backend.executeResult(result, executeAction);
            if (outcome.type === "done") {
              setFeedback({ tone: "success", message: outcome.message });
              if (outcome.refresh) onRefresh();
            }
          }
        }
        onSettled();
      } catch (error) {
        setFeedback({ tone: "error", message: describeError(error) });
      }
    },
    [onEngaged, onRefresh, onSettled, pending, setCopiedResultId, setFeedback, setPending, t],
  );

  const confirmingActionId = useCallback(
    (resultId: string | undefined) => (pending && pending.resultId === resultId ? pending.actionId : null),
    [pending],
  );

  return { run, feedback, showFeedback: setFeedback, copiedResultId, confirmingActionId };
}
