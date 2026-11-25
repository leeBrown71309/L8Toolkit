/**
 * Moves L8 Toolkit to its next version, following the project's numbering rule:
 *
 *   0.1.0 → 0.1.1 → … → 0.1.5 → 0.2.0 → 0.2.1 → … → 0.2.5 → 0.3.0 → … until 1.0.0
 *
 * The patch number counts up to 5, then the minor number goes up and the patch restarts at 0.
 * Moving to 1.0.0 is a decision, not a count: pass `--to 1.0.0` explicitly.
 *
 * Usage:
 *   bun scripts/bump-version.ts            # next version by the rule
 *   bun scripts/bump-version.ts --to 1.0.0 # a chosen version (must be higher)
 *
 * Updates package.json, src-tauri/tauri.conf.json and src-tauri/Cargo.toml; run `cargo check`
 * afterwards so Cargo.lock follows.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

/** Highest patch number before the minor number goes up. */
export const LAST_PATCH = 5;

const VERSION_PATTERN = /^(\d+)\.(\d+)\.(\d+)$/;

export function parseVersion(version: string): [number, number, number] {
  const match = VERSION_PATTERN.exec(version);
  if (!match) throw new Error(`Not a x.y.z version: ${version}`);
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

/** The version that follows `current` under the numbering rule. */
export function nextVersion(current: string): string {
  const [major, minor, patch] = parseVersion(current);
  return patch >= LAST_PATCH ? `${major}.${minor + 1}.0` : `${major}.${minor}.${patch + 1}`;
}

export function isHigher(candidate: string, current: string): boolean {
  const next = parseVersion(candidate);
  const now = parseVersion(current);
  for (let index = 0; index < 3; index += 1) {
    const difference = (next[index] ?? 0) - (now[index] ?? 0);
    if (difference !== 0) return difference > 0;
  }
  return false;
}

/** Replaces the first `"version": "…"` (JSON) or `version = "…"` (TOML) entry. */
export function replaceVersion(text: string, from: string, to: string, format: "json" | "toml"): string {
  const pattern = format === "json" ? /("version"\s*:\s*")([^"]+)(")/ : /^(version\s*=\s*")([^"]+)(")/m;
  const match = pattern.exec(text);
  if (!match || match[2] !== from) throw new Error(`Expected version ${from}, found ${match?.[2] ?? "none"}`);
  return text.replace(pattern, `$1${to}$3`);
}

const FILES: { path: string; format: "json" | "toml" }[] = [
  { path: "package.json", format: "json" },
  { path: "src-tauri/tauri.conf.json", format: "json" },
  { path: "src-tauri/Cargo.toml", format: "toml" },
];

export function main(argv: readonly string[], root = resolve(import.meta.dirname, "..")): string {
  const config = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8")) as { version: string };
  const current = config.version;
  const toIndex = argv.indexOf("--to");
  const target = toIndex === -1 ? nextVersion(current) : argv[toIndex + 1];
  if (!target) throw new Error("--to needs a version");
  if (!isHigher(target, current)) throw new Error(`${target} is not higher than ${current}`);
  for (const { path, format } of FILES) {
    const full = resolve(root, path);
    writeFileSync(full, replaceVersion(readFileSync(full, "utf8"), current, target, format), "utf8");
  }
  console.log(`L8 Toolkit ${current} → ${target}. Run "cargo check" in src-tauri so Cargo.lock follows.`);
  return target;
}

const invokedPath = process.argv[1];
if (invokedPath && import.meta.url === pathToFileURL(resolve(invokedPath)).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
