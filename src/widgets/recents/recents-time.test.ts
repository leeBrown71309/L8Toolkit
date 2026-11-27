import { describe, expect, test } from "bun:test";

import { isDragGesture, relativeAge } from "./recents-time";

describe("relativeAge", () => {
  const now = 10_000_000_000;

  test("counts minutes, hours, then days", () => {
    expect(relativeAge(now - 20_000, now)).toEqual({ key: "recents.time.now", count: 0 });
    expect(relativeAge(now - 5 * 60_000, now)).toEqual({ key: "recents.time.minutes", count: 5 });
    expect(relativeAge(now - 3 * 3_600_000, now)).toEqual({ key: "recents.time.hours", count: 3 });
    expect(relativeAge(now - 50 * 3_600_000, now)).toEqual({ key: "recents.time.days", count: 2 });
  });

  test("a clock moved back never shows a negative age", () => {
    expect(relativeAge(now + 60_000, now).key).toBe("recents.time.now");
  });
});

describe("isDragGesture", () => {
  test("a small wobble is still a click", () => {
    expect(isDragGesture(10, 10, 12, 12)).toBe(false);
    expect(isDragGesture(10, 10, 16, 10)).toBe(true);
  });
});
