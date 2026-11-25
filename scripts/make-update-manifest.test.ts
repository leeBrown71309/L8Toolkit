import { describe, expect, test } from "bun:test";

import { buildManifest, defaultInstallerUrl, installerAssetName, parseArgs } from "./make-update-manifest";

const VALID = {
  version: "0.2.0",
  notes: "  Audio bars.\n",
  pubDate: "2026-11-26T10:00:00Z",
  signature: "dW50cnVzdGVk\n",
  url: "https://github.com/leeBrown71309/L8Toolkit/releases/download/v0.2.0/L8Toolkit_0.2.0_x64-setup.exe",
};

describe("buildManifest", () => {
  test("produces the shape tauri-plugin-updater reads", () => {
    expect(buildManifest(VALID)).toEqual({
      version: "0.2.0",
      notes: "Audio bars.",
      pub_date: "2026-11-26T10:00:00.000Z",
      platforms: { "windows-x86_64": { signature: "dW50cnVzdGVk", url: VALID.url } },
    });
  });

  test("refuses manifests installed copies could not use", () => {
    expect(() => buildManifest({ ...VALID, version: "v0.2" })).toThrow("Invalid version");
    expect(() => buildManifest({ ...VALID, signature: " \n" })).toThrow("signature is empty");
    expect(() => buildManifest({ ...VALID, notes: "" })).toThrow("notes are empty");
    expect(() => buildManifest({ ...VALID, url: "http://example.com/a.exe" })).toThrow("HTTPS");
    expect(() => buildManifest({ ...VALID, pubDate: "yesterday" })).toThrow("Invalid date");
  });
});

describe("asset names", () => {
  test("have no spaces, so GitHub keeps them as is", () => {
    expect(installerAssetName("0.2.0")).toBe("L8Toolkit_0.2.0_x64-setup.exe");
    expect(defaultInstallerUrl("0.2.0")).toBe(VALID.url);
  });
});

describe("parseArgs", () => {
  test("reads name/value pairs and rejects the rest", () => {
    expect(parseArgs(["--version", "0.2.0", "--notes", "x"]).get("version")).toBe("0.2.0");
    expect(() => parseArgs(["--colour", "red"])).toThrow("Unknown option");
    expect(() => parseArgs(["--version"])).toThrow("Missing value");
    expect(() => parseArgs(["0.2.0"])).toThrow("Unexpected argument");
  });
});
