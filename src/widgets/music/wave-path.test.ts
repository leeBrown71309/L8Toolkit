import { describe, expect, test } from "bun:test";

import { layoutWave, THUMB_GAP } from "./wave-path";

const WIDTH = 400;
const HEIGHT = 14;

function yValues(path: string): number[] {
  return path
    .slice(1)
    .split("L")
    .map((pair) => Number(pair.split(" ")[1]));
}

describe("layoutWave", () => {
  test("splits the line at the thumb, with a gap on both sides", () => {
    const layout = layoutWave(WIDTH, HEIGHT, 0.5, 3, 0);
    expect(layout.thumbX).toBe(200);
    expect(layout.remaining).toBe(`M${200 + THUMB_GAP} 7H400`);
    const xs = layout.played.slice(1).split("L").map((pair) => Number(pair.split(" ")[0]));
    expect(Math.max(...xs)).toBe(200 - THUMB_GAP);
  });

  test("undulates while playing and lies flat when paused", () => {
    const playing = yValues(layoutWave(WIDTH, HEIGHT, 0.6, 3, 0).played);
    expect(Math.max(...playing) - Math.min(...playing)).toBeGreaterThan(5);
    const paused = yValues(layoutWave(WIDTH, HEIGHT, 0.6, 0, 0).played);
    expect(new Set(paused)).toEqual(new Set([7]));
  });

  test("meets the thumb on the middle line", () => {
    const ys = yValues(layoutWave(WIDTH, HEIGHT, 0.6, 3, 1.3).played);
    expect(ys.at(-1)).toBe(7);
  });

  test("handles the two ends of the track", () => {
    expect(layoutWave(WIDTH, HEIGHT, 0, 3, 0).played).toBe("");
    expect(layoutWave(WIDTH, HEIGHT, 1, 3, 0).remaining).toBe("");
    expect(layoutWave(WIDTH, HEIGHT, 1.4, 3, 0).thumbX).toBe(WIDTH);
  });
});
