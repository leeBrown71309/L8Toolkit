import { describe, expect, test } from "bun:test";

import { tabForKeyCode, tabNumber } from "./tab-shortcuts";

describe("tabForKeyCode", () => {
  test("reads the physical digit row and the numpad", () => {
    expect(tabForKeyCode("Digit1")).toBe("search");
    expect(tabForKeyCode("Digit3")).toBe("timer");
    expect(tabForKeyCode("Numpad4")).toBe("recents");
  });

  test("ignores other keys and digits without a widget", () => {
    expect(tabForKeyCode("KeyA")).toBeUndefined();
    expect(tabForKeyCode("Digit0")).toBeUndefined();
    expect(tabForKeyCode("Digit9")).toBeUndefined();
  });
});

describe("tabNumber", () => {
  test("matches the shortcut digit", () => {
    expect(tabNumber("music")).toBe(2);
    expect(tabForKeyCode(`Digit${tabNumber("recents")}`)).toBe("recents");
  });
});
