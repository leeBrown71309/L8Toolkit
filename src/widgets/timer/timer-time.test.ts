import { describe, expect, test } from "bun:test";

import type { TimerSnapshot } from "../../shared/types";
import { elapsedRatio, formatClock, formatCompact, remainingMs } from "./timer-time";

const RUNNING: TimerSnapshot = {
  mode: "countdown",
  status: "running",
  phase: null,
  durationMs: 60_000,
  remainingMs: 60_000,
  endsAtMs: 100_000,
  completedFocus: 0,
  rounds: 4,
  updatedAtMs: 40_000,
};

describe("formatClock", () => {
  test("rounds seconds up so 0:00 means the time is up", () => {
    expect(formatClock(299_001)).toBe("5:00");
    expect(formatClock(1)).toBe("0:01");
    expect(formatClock(0)).toBe("0:00");
  });

  test("shows hours only when needed", () => {
    expect(formatClock(3_723_000)).toBe("1:02:03");
  });
});

describe("formatCompact", () => {
  test("counts minutes, then seconds in the last minute", () => {
    expect(formatCompact(24 * 60_000 + 1)).toBe("25");
    expect(formatCompact(59_000)).toBe("59");
    expect(formatCompact(150 * 60_000)).toBe("2h");
  });
});

describe("remaining time", () => {
  test("follows the deadline while running", () => {
    expect(remainingMs(RUNNING, 70_000)).toBe(30_000);
    expect(remainingMs(RUNNING, 120_000)).toBe(0);
    expect(elapsedRatio(RUNNING, 70_000)).toBe(0.5);
  });

  test("stays put while paused", () => {
    const paused: TimerSnapshot = { ...RUNNING, status: "paused", endsAtMs: null, remainingMs: 12_000 };
    expect(remainingMs(paused, 999_999)).toBe(12_000);
  });
});
