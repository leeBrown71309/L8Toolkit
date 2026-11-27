import { ArrowCounterClockwise, Coffee, Pause, Play, Plus, SkipForward, Target, Timer } from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type WheelEvent } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend } from "../../shared/backend";
import type { TimerMode, TimerRequest, TimerSettings, TimerSnapshot } from "../../shared/types";
import { useTimerClock } from "./timer-state";
import { elapsedRatio, formatClock, isTimerActive, phaseLabelKey, timerColor } from "./timer-time";

import "../shared/transport-button.css";
import "./timer-panel.css";

const MINUTE_MS = 60_000;
const PRESET_MINUTES = [1, 3, 5, 10, 15, 25, 45, 60];
const MAX_MINUTES = 24 * 60;
const DIAL_SIZE = 92;
const DIAL_STROKE = 5;
const DIAL_RADIUS = (DIAL_SIZE - DIAL_STROKE) / 2;
const DIAL_CIRCUMFERENCE = 2 * Math.PI * DIAL_RADIUS;

interface TimerPanelProps {
  timer: TimerSnapshot | null;
  settings: TimerSettings;
  isActive: boolean;
  onClose: (restoreFocus: boolean) => void;
  onEngaged: () => void;
}

/**
 * The Timer tab: a countdown or a Pomodoro. Rust owns the running timer
 * (timer/mod.rs), so it keeps counting with the notch closed; this panel
 * only sends requests and draws the state it receives.
 */
export function TimerPanel({ timer, settings, isActive, onClose, onEngaged }: TimerPanelProps) {
  const { t } = useI18n();
  const rootRef = useRef<HTMLDivElement>(null);
  const [mode, setMode] = useState<TimerMode>("countdown");
  const [minutes, setMinutes] = useState(5);
  const remaining = useTimerClock(timer);
  const active = isTimerActive(timer);

  useEffect(() => {
    if (isActive) rootRef.current?.focus({ preventScroll: true });
  }, [isActive]);

  // An idle countdown offers its last length again.
  const lastDuration = timer?.status === "idle" ? timer.durationMs : null;
  useEffect(() => {
    if (lastDuration) setMinutes(Math.max(1, Math.round(lastDuration / MINUTE_MS)));
  }, [lastDuration]);

  const send = useCallback(
    (request: TimerRequest) => {
      onEngaged();
      backend.timerCommand(request).catch((error: unknown) => console.error("Timer request failed", error));
    },
    [onEngaged],
  );

  const start = () =>
    send(mode === "pomodoro" ? { type: "startPomodoro" } : { type: "startCountdown", durationMs: minutes * MINUTE_MS });
  const adjust = (step: number) => {
    onEngaged();
    if (active) send({ type: "addTime", deltaMs: step * MINUTE_MS });
    else setMinutes((current) => Math.min(MAX_MINUTES, Math.max(1, current + step)));
  };
  const togglePause = () => {
    if (!timer) return;
    send({ type: timer.status === "running" ? "pause" : "resume" });
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    onEngaged();
    if (event.key === "Escape") {
      event.preventDefault();
      onClose(true);
      return;
    }
    // Alt+arrows switch widgets.
    if (event.altKey) return;
    const handled = (() => {
      if (event.key === " " || event.key === "Enter") {
        if (active) togglePause();
        else start();
      } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        if (active || mode === "countdown") adjust(event.key === "ArrowUp" ? 1 : -1);
      } else if (event.key === "Backspace" || event.key === "Delete") {
        if (active) send({ type: "reset" });
      } else if (event.key === "ArrowRight" && event.ctrlKey) {
        if (timer?.mode === "pomodoro" && active) send({ type: "skip" });
      } else {
        return false;
      }
      return true;
    })();
    if (handled) event.preventDefault();
  };

  const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
    if (active || mode !== "countdown" || event.deltaY === 0) return;
    adjust(event.deltaY < 0 ? 1 : -1);
  };

  const phase = active ? timer.phase : mode === "pomodoro" ? "focus" : null;
  const color = timerColor(phase);
  const shownMs = active ? remaining : mode === "pomodoro" ? settings.focusMinutes * MINUTE_MS : minutes * MINUTE_MS;
  const progress = active ? elapsedRatio(timer) : 0;
  const paused = timer?.status === "paused";

  return (
    <div
      ref={rootRef}
      className="timer"
      tabIndex={-1}
      style={{ "--timer-color": color } as React.CSSProperties}
      data-state={active ? timer.status : "idle"}
      aria-label={t("widget.timer")}
      onKeyDown={handleKeyDown}
    >
      <div className="timer__layout">
        <Dial progress={progress} phase={phase} countdown={active ? timer.mode === "countdown" : mode === "countdown"} />
        <div className="timer__details">
          <div className="timer__heading">
            {active ? (
              <ActiveHeading timer={timer} paused={paused} />
            ) : (
              <ModeSwitch
                value={mode}
                onChange={(next) => {
                  onEngaged();
                  setMode(next);
                }}
              />
            )}
          </div>
          <div className="timer__row">
            <div
              className="timer__digits"
              onWheel={handleWheel}
              title={!active && mode === "countdown" ? t("timer.adjustHint") : undefined}
              aria-live="off"
            >
              {formatClock(shownMs)}
            </div>
            <div className="timer__controls">
              {active ? (
                <>
                  <button
                    type="button"
                    className="transport-button"
                    aria-label={t("timer.reset")}
                    title={`${t("timer.reset")} (${t("keys.delete")})`}
                    onClick={() => send({ type: "reset" })}
                  >
                    <ArrowCounterClockwise size={16} weight="bold" />
                  </button>
                  <button
                    type="button"
                    className="transport-button transport-button--primary timer__primary"
                    aria-label={paused ? t("timer.resume") : t("timer.pause")}
                    title={`${paused ? t("timer.resume") : t("timer.pause")} (${t("keys.space")})`}
                    onClick={togglePause}
                  >
                    <Pause className="transport-button__icon" data-visible={!paused} size={17} weight="fill" />
                    <Play className="transport-button__icon" data-visible={paused} size={17} weight="fill" />
                  </button>
                  {timer.mode === "pomodoro" ? (
                    <button
                      type="button"
                      className="transport-button"
                      aria-label={t("timer.skip")}
                      title={`${t("timer.skip")} (Ctrl+→)`}
                      onClick={() => send({ type: "skip" })}
                    >
                      <SkipForward size={16} weight="fill" />
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="transport-button timer__add"
                      aria-label={t("timer.addMinute")}
                      title={`${t("timer.addMinute")} (↑)`}
                      onClick={() => adjust(1)}
                    >
                      <Plus size={11} weight="bold" />1
                    </button>
                  )}
                </>
              ) : (
                <button type="button" className="timer__start" onClick={start}>
                  <Play size={14} weight="fill" />
                  {t("timer.start")}
                </button>
              )}
            </div>
          </div>
          {!active && mode === "countdown" && (
            <div className="timer__presets" role="group" aria-label={t("timer.presets")}>
              {PRESET_MINUTES.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  className="timer__preset"
                  data-selected={preset === minutes}
                  onClick={() => {
                    onEngaged();
                    setMinutes(preset);
                  }}
                >
                  {t("timer.minutes", { count: String(preset) })}
                </button>
              ))}
            </div>
          )}
          {!active && mode === "pomodoro" && (
            <p className="timer__plan">
              {t("timer.plan", {
                focus: String(settings.focusMinutes),
                short: String(settings.shortBreakMinutes),
                long: String(settings.longBreakMinutes),
                rounds: String(settings.roundsBeforeLongBreak),
              })}
            </p>
          )}
          {active && timer.mode === "pomodoro" && <Rounds timer={timer} />}
        </div>
      </div>
    </div>
  );
}

function Dial({ progress, phase, countdown }: { progress: number; phase: TimerSnapshot["phase"]; countdown: boolean }) {
  const Icon = countdown ? Timer : phase === "focus" ? Target : Coffee;
  return (
    <div className="timer__dial" aria-hidden="true">
      <svg className="timer__dial-ring" width={DIAL_SIZE} height={DIAL_SIZE} viewBox={`0 0 ${DIAL_SIZE} ${DIAL_SIZE}`}>
        <circle className="timer__dial-track" cx={DIAL_SIZE / 2} cy={DIAL_SIZE / 2} r={DIAL_RADIUS} />
        {/* Starts full and empties clockwise as time runs out. */}
        <circle
          className="timer__dial-fill"
          cx={DIAL_SIZE / 2}
          cy={DIAL_SIZE / 2}
          r={DIAL_RADIUS}
          strokeDasharray={DIAL_CIRCUMFERENCE}
          strokeDashoffset={-DIAL_CIRCUMFERENCE * progress}
        />
      </svg>
      <span key={`${countdown}-${phase}`} className="timer__dial-icon">
        <Icon size={26} weight="duotone" />
      </span>
    </div>
  );
}

function ActiveHeading({ timer, paused }: { timer: TimerSnapshot; paused: boolean }) {
  const { t } = useI18n();
  return (
    <>
      <span className="timer__label">{t(phaseLabelKey(timer.mode === "pomodoro" ? timer.phase : null))}</span>
      {paused && <span className="timer__badge">{t("timer.paused")}</span>}
    </>
  );
}

function ModeSwitch({ value, onChange }: { value: TimerMode; onChange: (mode: TimerMode) => void }) {
  const { t } = useI18n();
  const modes: TimerMode[] = ["countdown", "pomodoro"];
  return (
    <div className="timer__modes" role="radiogroup" aria-label={t("timer.mode")} data-value={value}>
      <span className="timer__modes-indicator" aria-hidden="true" />
      {modes.map((mode) => (
        <button
          key={mode}
          type="button"
          role="radio"
          aria-checked={value === mode}
          className="timer__mode"
          onClick={() => onChange(mode)}
        >
          {t(mode === "countdown" ? "timer.countdown" : "timer.pomodoro")}
        </button>
      ))}
    </div>
  );
}

/** Focus sessions of the current cycle, as dots. */
function Rounds({ timer }: { timer: TimerSnapshot }) {
  const { t } = useI18n();
  const rounds = Math.max(1, timer.rounds);
  const completedInCycle = timer.completedFocus % rounds;
  // During the long break the cycle is complete: every dot is filled.
  const filled = timer.phase === "longBreak" ? rounds : completedInCycle;
  const current = timer.phase === "focus" ? completedInCycle + 1 : null;
  return (
    <div
      className="timer__rounds"
      aria-label={t("timer.round", { current: String(current ?? filled), total: String(rounds) })}
    >
      {Array.from({ length: rounds }, (_, index) => (
        <span key={index} className="timer__round" data-filled={index < filled} data-current={current === index + 1} />
      ))}
      <span className="timer__rounds-text">
        {t("timer.round", { current: String(current ?? filled), total: String(rounds) })}
      </span>
    </div>
  );
}
