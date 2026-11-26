import { describe, expect, test } from "bun:test";

import { ACCENT_VISIBLE_CELLS, accentBarWidth, accentScrollOffset } from "./accent-bar";

describe("accent bar layout", () => {
  test("short rows show every character without scrolling", () => {
    expect(accentScrollOffset(4, 3)).toBe(0);
    expect(accentBarWidth(4)).toBe(4 * 40 + 24);
  });

  test("long rows are capped and keep the selection near the middle", () => {
    const count = 30;
    expect(accentBarWidth(count)).toBe(ACCENT_VISIBLE_CELLS * 40 + 24);
    expect(accentScrollOffset(count, 0)).toBe(0);
    expect(accentScrollOffset(count, 7)).toBe(0);
    expect(accentScrollOffset(count, 12)).toBe(5);
    expect(accentScrollOffset(count, count - 1)).toBe(count - ACCENT_VISIBLE_CELLS);
  });
});
