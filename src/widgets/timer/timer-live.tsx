import { Coffee, Target, Timer } from "@phosphor-icons/react";

import { useI18n } from "../../i18n/i18n-context";
import type { PomodoroPhase, TimerFinished, TimerSnapshot } from "../../shared/types";
import type { LiveMode } from "../music/now-playing-live";
import { useTimerClock } from "./timer-state";
import { formatClock, formatCompact, phaseLabelKey, remainingMs } from "./timer-time";

import "./timer-live.css";

function PhaseIcon({ phase, size }: { phase: PomodoroPhase | null; size: number }) {
  if (phase === "focus") return <Target size={size} weight="bold" />;
  if (phase === "shortBreak" || phase === "longBreak") return <Coffee size={size} weight="bold" />;
  return <Timer size={size} weight="bold" />;
}

interface TimerLiveProps {
  timer: TimerSnapshot | null;
  /** A period just ended: the notch peeks to say so. */
  alert: TimerFinished | null;
  mode: LiveMode;
}

/**
 * The timer inside the black notch: icon and time left while folded (the
 * edge light traces the elapsed time), a short message when a period ends.
 */
export function TimerLive({ timer, alert, mode }: TimerLiveProps) {
  const { t } = useI18n();
  const remaining = useTimerClock(timer);
  const phase = timer?.mode === "pomodoro" ? timer.phase : null;
  const shownMode = alert ? "peek" : mode;

  return (
    <div className="timer-live" data-mode={shownMode} data-paused={timer?.status === "paused"} aria-hidden={shownMode === "open"}>
      <span className="timer-live__icon">
        <PhaseIcon phase={alert ? (alert.nextPhase ?? null) : phase} size={13} />
      </span>
      <span className="timer-live__time">{formatClock(remaining)}</span>
      {alert && (
        <span className="timer-live__alert">
          <span className="timer-live__badge">
            <PhaseIcon phase={alert.phase} size={20} />
          </span>
          <span className="timer-live__text">
            <span className="timer-live__title">{t(alertTitleKey(alert))}</span>
            {alert.nextPhase && (
              <span className="timer-live__subtitle">
                {t(alert.continued ? "timer.next.started" : "timer.next.ready", { phase: t(phaseLabelKey(alert.nextPhase)) })}
              </span>
            )}
          </span>
        </span>
      )}
    </div>
  );
}

function alertTitleKey(alert: TimerFinished) {
  if (alert.mode === "countdown") return "timer.done.title" as const;
  return alert.phase === "focus" ? ("timer.done.focus" as const) : ("timer.done.break" as const);
}

/** The timer in the small bubble beside the notch: minutes left. */
export function TimerBubbleContent({ timer }: { timer: TimerSnapshot }) {
  useTimerClock(timer);
  return <span className="timer-bubble">{formatCompact(remainingMs(timer))}</span>;
}
