# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project

L8 Toolkit: a Windows tray app hosting small widgets in a virtual notch at the top of the screen
(Search and Music today; macOS and Linux later). Tauri 2 (Rust) + React 19 / TypeScript / Vite,
Bun as package manager. Public repository: https://github.com/leeBrown71309/L8Toolkit (MIT).

The maintainer reads French: answer in French. Code, comments, commits and docs stay in English.

## Commands

```powershell
bun install
bun run app:dev          # the TEST copy ("L8 Toolkit Dev"), hot reload; stops the installed release first
bun run typecheck        # tsc
bun run test             # bun tests of the release scripts
bun run build            # typecheck + vite build
cd src-tauri; cargo test; cargo clippy --all-targets -- -D warnings
bun run dev:remove       # remove the test copy from this PC (keeps its data, see below)
bun run release:install  # install / update the published release on this PC
```

After any change run `bun x tsc -p tsconfig.json --noEmit`; after a feature also `bun run build`,
`cargo test` and `cargo clippy --all-targets -- -D warnings`. CI (`.github/workflows/ci.yml`)
runs the same checks on every pull request into `main`.

## Layout

- `src-tauri/src/`: Rust core. `notch.rs` (window, hover, fullscreen), `search/` (pure ranking,
  unit tested), `media.rs` (Windows media sessions), `audio/` (loopback capture + FFT for the notch
  bars), `updates.rs` (in-app updater), `i18n/` (Rust-side texts), `platform/` (all Win32 code;
  other OSes get stubs in `platform/unsupported.rs`).
- `src/`: React UI, one bundle for two windows (notch, settings). `widgets/notch`,
  `widgets/spotlight`, `widgets/music`, `settings/`, `shared/` (typed IPC in `backend.ts`, browser
  preview mock in `preview-backend.ts`).
- `scripts/`: release and dev-copy PowerShell scripts, update manifest generator.
- `docs/releasing.md`: the full release procedure and the signing key.

## Conventions

- Every user-visible string exists in English and French: `src/i18n/messages/en.ts` + `fr.ts`
  (typed, a missing key does not compile) and `src-tauri/src/i18n/` (`Text` enum, exhaustive matches).
- Serde payloads use camelCase; struct-variant enums need `rename_all_fields = "camelCase"`.
- Windows' native Mica/Acrylic ignore window regions: the notch draws its materials itself.
- UI work follows the `design-taste-frontend` and `emil-design-eng` skills; animations stay on
  `transform` / `opacity` / registered custom properties.
- Commits: `<type>(<scope>): <message>`, no AI attribution anywhere.
- Quick Accent ignores simulated keys. A test copy started with `L8_ACCEPT_SIMULATED_KEYS=1` accepts
  them (debug builds only) and logs its decisions at debug level, never the letters typed. Never
  send simulated keys unless the test window is confirmed to have the focus: Windows often refuses
  the focus to a background process, and the keys then land in the maintainer's own windows.

## Two copies of the app: test and release

| | Release | Test copy |
| --- | --- | --- |
| Name | L8 Toolkit | L8 Toolkit Dev (blue notch icon) |
| Identifier / data | `com.l8.toolkit` | `com.l8.toolkit.dev` |
| Started by | installer, Start menu, sign-in | `bun run app:dev` (`src-tauri/tauri.dev.conf.json`) |

Only one copy may run at a time on the maintainer's PC (both draw a notch and use the same shortcut).

### Rule: before publishing a release

Whenever a release is about to go out (a pull request into `main` that bumps the version, a manual
run of the Release workflow, or a local `bun run app:build`):

1. Stop every process of the test copy: the `bun run app:dev` / `tauri dev` task, its Vite server
   on port 1420, and any `l8-tools-kits.exe` started from `src-tauri\target`.
2. Remove the test copy from the PC: `bun run dev:remove` (startup entry, installed test copy if
   any, built executables in `src-tauri\target`).
3. **Keep its data** (`%APPDATA%\com.l8.toolkit.dev`, `%LOCALAPPDATA%\com.l8.toolkit.dev`) so the
   next development session does not start from scratch. Delete it only when the maintainer asks
   (`powershell -File scripts/remove-dev-app.ps1 -DeleteData`).
4. Once the release is published, install it with `bun run release:install` (it downloads the
   installer from the GitHub release: ask the maintainer before running it).

## Version numbers

The patch number counts from 0 to 5, then the minor number goes up and the patch restarts:
`0.1.0 → 0.1.1 → … → 0.1.5 → 0.2.0 → … → 0.2.5 → 0.3.0 → …` until the maintainer decides on 1.0.0.
`bun run version:next` applies the next number to `package.json`, `src-tauri/tauri.conf.json` and
`src-tauri/Cargo.toml` (then `cargo check` so `Cargo.lock` follows); `--to 1.0.0` for a chosen one.
Every release bumps the version; a merge without a bump publishes nothing.

## Releases

Work on `dev`, open a pull request into `main` (protected: one approving review from the
maintainer's second account `blee71409`, CI green, linear history). Merging a bumped version
publishes a signed release; installed copies update from Settings › Updates. Details, the signing
key and its GitHub secrets: `docs/releasing.md`. Never handle the signing key password: the
maintainer types it in their own terminal (`scripts/setup-update-signing.ps1`).
