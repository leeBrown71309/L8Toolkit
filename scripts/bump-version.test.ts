import { describe, expect, test } from "bun:test";

import { isHigher, nextVersion, replaceVersion } from "./bump-version";

describe("nextVersion", () => {
  test("counts the patch number up to 5", () => {
    expect(nextVersion("0.1.0")).toBe("0.1.1");
    expect(nextVersion("0.1.4")).toBe("0.1.5");
  });

  test("moves to the next minor number after .5", () => {
    expect(nextVersion("0.1.5")).toBe("0.2.0");
    expect(nextVersion("0.2.5")).toBe("0.3.0");
    expect(nextVersion("0.9.5")).toBe("0.10.0");
  });

  test("rejects anything that is not x.y.z", () => {
    expect(() => nextVersion("0.2")).toThrow("Not a x.y.z version");
  });
});

describe("isHigher", () => {
  test("compares numerically, not as text", () => {
    expect(isHigher("0.10.0", "0.9.5")).toBe(true);
    expect(isHigher("1.0.0", "0.9.5")).toBe(true);
    expect(isHigher("0.1.0", "0.1.0")).toBe(false);
    expect(isHigher("0.1.0", "0.1.1")).toBe(false);
  });
});

describe("replaceVersion", () => {
  test("changes only the version entry", () => {
    const json = '{\n  "name": "x",\n  "version": "0.1.0",\n  "dep": { "version": "9.9.9" }\n}';
    expect(replaceVersion(json, "0.1.0", "0.1.1", "json")).toContain('"version": "0.1.1"');
    expect(replaceVersion(json, "0.1.0", "0.1.1", "json")).toContain('"version": "9.9.9"');
    const toml = '[package]\nname = "x"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }';
    expect(replaceVersion(toml, "0.1.0", "0.1.1", "toml")).toContain('version = "0.1.1"');
  });

  test("refuses to touch a file that is not at the expected version", () => {
    expect(() => replaceVersion('{ "version": "0.2.0" }', "0.1.0", "0.1.1", "json")).toThrow("Expected version 0.1.0");
  });
});
