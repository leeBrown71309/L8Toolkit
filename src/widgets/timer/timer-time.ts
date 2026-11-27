import type { MessageKey } from "../../i18n/messages/en";
import type { PomodoroPhase, TimerSnapshot } from "../../shared/types";

/** Breaks are drawn in a calm green; focus and plain countdowns in the accent. */
export const BREAK_COLOR = "#5fcb94";
const FOCUS_COLOR = "var(--accent)";

export function isTimerActive(timer: TimerSnapshot | null): timer is TimerSnapshot {
  return timer !== null && timer.status !== "idle";
}

/** Time left "now": derived from the deadline while running, never polled. */
export function remainingMs(timer: TimerSnapshot, now: number = Date.now()): number {
  return timer.status === "running" && timer.endsAtMs !== null ? Math.max(0, timer.endsAtMs - now) : timer.remainingMs;
}

/** Elapsed share of the current period, from 0 to 1. */
export function elapsedRatio(timer: TimerSnapshot, now: number = Date.now()): number {
  if (timer.durationMs <= 0) return 0;
  return Math.min(1, Math.max(0, 1 - remainingMs(timer, now) / timer.durationMs));
}

/**
 * "4:59", "1:02:03": seconds are rounded up, so the display reaches 0:00
 * exactly when the time is up, not a second early.
 */
export function formatClock(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.ceil(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

/** The shortest reading for the small bubble: minutes left, seconds in the last minute. */
export function formatCompact(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.ceil(milliseconds / 1000));
  if (totalSeconds < 60) return String(totalSeconds);
  const minutes = Math.ceil(totalSeconds / 60);
  return minutes >= 100 ? `${Math.floor(minutes / 60)}h` : String(minutes);
}

export function timerColor(phase: PomodoroPhase | null): string {
  return phase === "shortBreak" || phase === "longBreak" ? BREAK_COLOR : FOCUS_COLOR;
}

export function phaseLabelKey(phase: PomodoroPhase | null): MessageKey {
  if (phase === "focus") return "timer.phase.focus";
  if (phase === "shortBreak") return "timer.phase.shortBreak";
  if (phase === "longBreak") return "timer.phase.longBreak";
  return "timer.countdown";
}
