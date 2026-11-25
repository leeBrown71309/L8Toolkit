/**
 * Builds the `latest.json` manifest read by tauri-plugin-updater for a GitHub release.
 *
 * Usage:
 *   bun scripts/make-update-manifest.ts --version 0.2.0 --notes-file release-notes.md
 *
 * By default the signature is read from `release-assets/L8Toolkit_<version>_x64-setup.exe.sig`
 * (the release workflow copies the installer there under a name without spaces, because
 * GitHub would rewrite spaces in asset names) and the installer URL points at the matching
 * GitHub release asset. `--sig`, `--url`, `--out` and `--pub-date` override the defaults.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const GITHUB_REPOSITORY = "leeBrown71309/L8Toolkit";
export const ASSET_DIRECTORY = "release-assets";
export const DEFAULT_OUTPUT = "latest.json";
export const WINDOWS_PLATFORM = "windows-x86_64";

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const OPTION_NAMES = new Set(["version", "notes", "notes-file", "sig", "url", "out", "pub-date"]);

export interface UpdateManifest {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<typeof WINDOWS_PLATFORM, { signature: string; url: string }>;
}

export interface ManifestInput {
  version: string;
  notes: string;
  pubDate: string;
  signature: string;
  url: string;
}

/** File name of the installer as published on the release (no spaces). */
export function installerAssetName(version: string): string {
  return `L8Toolkit_${version}_x64-setup.exe`;
}

export function defaultInstallerUrl(version: string): string {
  return `https://github.com/${GITHUB_REPOSITORY}/releases/download/v${version}/${installerAssetName(version)}`;
}

export function defaultSignaturePath(version: string, workingDirectory = process.cwd()): string {
  return resolve(workingDirectory, ASSET_DIRECTORY, `${installerAssetName(version)}.sig`);
}

/** Refuses anything that would publish a manifest installed copies cannot use. */
export function buildManifest({ version, notes, pubDate, signature, url }: ManifestInput): UpdateManifest {
  if (!VERSION_PATTERN.test(version)) throw new Error(`Invalid version: ${version}`);
  const cleanedSignature = signature.trim();
  if (cleanedSignature === "") throw new Error("The update signature is empty");
  if (notes.trim() === "") throw new Error("Release notes are empty");
  if (!url.startsWith("https://")) throw new Error(`The installer URL must use HTTPS: ${url}`);
  const date = new Date(pubDate);
  if (Number.isNaN(date.getTime())) throw new Error(`Invalid date: ${pubDate}`);
  return {
    version,
    notes: notes.trim(),
    pub_date: date.toISOString(),
    platforms: { [WINDOWS_PLATFORM]: { signature: cleanedSignature, url } },
  };
}

/** Parses `--name value` pairs; unknown or incomplete options fail instead of being ignored. */
export function parseArgs(argv: readonly string[]): Map<string, string> {
  const parsed = new Map<string, string>();
  for (let index = 0; index < argv.length; index += 2) {
    const argument = argv[index];
    if (!argument?.startsWith("--")) throw new Error(`Unexpected argument: ${String(argument)}`);
    const name = argument.slice(2);
    if (!OPTION_NAMES.has(name)) throw new Error(`Unknown option: --${name}`);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) throw new Error(`Missing value for --${name}`);
    parsed.set(name, value);
  }
  return parsed;
}

export function main(argv: readonly string[], workingDirectory = process.cwd()): void {
  const args = parseArgs(argv);
  const version = args.get("version");
  if (!version) throw new Error("--version is required");
  const notesFile = args.get("notes-file");
  const notes = args.get("notes") ?? (notesFile ? readFileSync(resolve(workingDirectory, notesFile), "utf8") : "");
  const signaturePath = args.get("sig") ?? defaultSignaturePath(version, workingDirectory);
  const manifest = buildManifest({
    version,
    notes,
    pubDate: args.get("pub-date") ?? new Date().toISOString(),
    signature: readFileSync(signaturePath, "utf8"),
    url: args.get("url") ?? defaultInstallerUrl(version),
  });
  const outputPath = resolve(workingDirectory, args.get("out") ?? DEFAULT_OUTPUT);
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  console.log(`Wrote ${outputPath} for L8 Toolkit ${manifest.version}.`);
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
