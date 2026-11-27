import { useEffect, useState } from "react";

import { backend, BACKEND_EVENTS } from "../../shared/backend";
import type { TimerSnapshot } from "../../shared/types";
import { useBackendEvent } from "../../shared/use-backend-event";
import { remainingMs } from "./timer-time";

const CLOCK_TICK_MS = 250;

/** Timer state, kept in sync with timer/mod.rs. */
export function useTimer(enabled: boolean): TimerSnapshot | null {
  const [timer, setTimer] = useState<TimerSnapshot | null>(null);

  useEffect(() => {
    if (!enabled) {
      setTimer(null);
      return;
    }
    let alive = true;
    backend
      .timerState()
      .then((state) => {
        if (alive) setTimer(state);
      })
      .catch((error: unknown) => console.error("Cannot read the timer state", error));
    return () => {
      alive = false;
    };
  }, [enabled]);

  useBackendEvent<TimerSnapshot>(BACKEND_EVENTS.timerState, (next) => {
    if (enabled) setTimer(next);
  });

  return enabled ? timer : null;
}

/** Re-renders a few times a second while running, for the digits. */
export function useTimerClock(timer: TimerSnapshot | null): number {
  const [, setTick] = useState(0);
  const running = timer?.status === "running";

  useEffect(() => {
    if (!running) return;
    const interval = window.setInterval(() => setTick((tick) => tick + 1), CLOCK_TICK_MS);
    return () => window.clearInterval(interval);
  }, [running]);

  return timer ? remainingMs(timer) : 0;
}
