import type { PomodoroPhase, TimerFinished, TimerRequest, TimerSettings, TimerSnapshot } from "./types";

const MINUTE_MS = 60_000;

/**
 * Browser-preview stand-in for timer/machine.rs: enough behaviour to design
 * the Timer tab and the notch (the real rules are tested in Rust). Speed it
 * up from the console with `?timerSpeed=30` in the URL.
 */
export function createPreviewTimer(
  readSettings: () => TimerSettings,
  emit: (event: string, payload: unknown) => void,
) {
  const speed = Number(new URLSearchParams(location.search).get("timerSpeed")) || 1;
  let state: TimerSnapshot = {
    mode: "countdown",
    status: "idle",
    phase: null,
    durationMs: 5 * MINUTE_MS,
    remainingMs: 5 * MINUTE_MS,
    endsAtMs: null,
    completedFocus: 0,
    rounds: 4,
    updatedAtMs: Date.now(),
  };
  let timeout: number | undefined;

  const phaseLength = (phase: PomodoroPhase) => {
    const settings = readSettings();
    const minutes =
      phase === "focus" ? settings.focusMinutes : phase === "shortBreak" ? settings.shortBreakMinutes : settings.longBreakMinutes;
    return minutes * MINUTE_MS;
  };

  const remaining = () => (state.status === "running" && state.endsAtMs ? Math.max(0, state.endsAtMs - Date.now()) : state.remainingMs);

  const publish = () => {
    state = { ...state, remainingMs: remaining(), updatedAtMs: Date.now() };
    window.clearTimeout(timeout);
    if (state.status === "running" && state.endsAtMs) {
      timeout = window.setTimeout(finish, (state.endsAtMs - Date.now()) / speed);
    }
    emit("timer:state", state);
    return state;
  };

  const run = (durationMs: number, remainingMs = durationMs) => {
    state = { ...state, status: "running", durationMs, remainingMs, endsAtMs: Date.now() + remainingMs };
  };

  const finish = () => {
    if (state.mode === "countdown" || !state.phase) {
      state = { ...state, status: "idle", endsAtMs: null, remainingMs: state.durationMs };
      emit("timer:finished", { mode: "countdown", phase: null, nextPhase: null, continued: false } satisfies TimerFinished);
      publish();
      return;
    }
    const ended = state.phase;
    const completedFocus = state.completedFocus + (ended === "focus" ? 1 : 0);
    const next: PomodoroPhase =
      ended !== "focus" ? "focus" : completedFocus % Math.max(1, state.rounds) === 0 ? "longBreak" : "shortBreak";
    const continued = readSettings().autoContinue;
    state = { ...state, phase: next, completedFocus };
    if (continued) run(phaseLength(next));
    else state = { ...state, status: "paused", durationMs: phaseLength(next), remainingMs: phaseLength(next), endsAtMs: null };
    emit("timer:finished", { mode: "pomodoro", phase: ended, nextPhase: next, continued } satisfies TimerFinished);
    publish();
  };

  const apply = (request: TimerRequest): TimerSnapshot => {
    switch (request.type) {
      case "startCountdown":
        state = { ...state, mode: "countdown", phase: null, completedFocus: 0 };
        run(request.durationMs);
        break;
      case "startPomodoro":
        state = { ...state, mode: "pomodoro", phase: "focus", completedFocus: 0, rounds: readSettings().roundsBeforeLongBreak };
        run(phaseLength("focus"));
        break;
      case "pause":
        if (state.status === "running") state = { ...state, status: "paused", remainingMs: remaining(), endsAtMs: null };
        break;
      case "resume":
        if (state.status === "paused") run(state.durationMs, state.remainingMs);
        break;
      case "reset":
        state = { ...state, mode: "countdown", status: "idle", phase: null, endsAtMs: null, completedFocus: 0 };
        state.remainingMs = state.durationMs;
        break;
      case "addTime": {
        const next = Math.max(1000, remaining() + request.deltaMs);
        state = { ...state, durationMs: state.durationMs + (next - remaining()) };
        if (state.status === "running") state.endsAtMs = Date.now() + next;
        else state.remainingMs = next;
        break;
      }
      case "skip":
        if (state.endsAtMs) state.endsAtMs = Date.now();
        finish();
        return state;
    }
    return publish();
  };

  return { snapshot: () => ({ ...state, remainingMs: remaining(), updatedAtMs: Date.now() }), apply };
}
