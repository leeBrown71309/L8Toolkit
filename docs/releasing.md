# Releasing L8 Toolkit

Releases are built and published by GitHub Actions; installed copies update themselves from
GitHub Releases. Nothing in the chain is paid.

## Branches and checks

- Work happens on `dev` (or feature branches) and reaches `main` through a pull request.
- `main` is protected by the "Protect main" ruleset: no direct push, no force push or deletion,
  linear history, one approving review, resolved conversations, and two required checks from
  `.github/workflows/ci.yml`: **Frontend (typecheck, tests, build)** and **Rust (tests, clippy)**.

## Publishing a version

1. In the pull request, bump the version in `package.json`, `src-tauri/tauri.conf.json` and
   `src-tauri/Cargo.toml` (then run `cargo check` once so `Cargo.lock` follows).
2. Replace `release-notes.md` with the notes of this version. They appear on the GitHub release
   and in Settings › Updates, so write them for users.
3. Merge. `.github/workflows/release.yml` builds the signed installer, writes `latest.json`
   and publishes the release `v<version>`. A version that already has a release is skipped,
   so merges that do not bump the version publish nothing.

Installed copies find it within 6 hours (or at their next start), show a Windows notification,
add "Install update" to the tray menu and a dot on Settings › Updates.

## On this PC around a release

Before a release goes out, the test copy must not be running or installed (see CLAUDE.md):

```powershell
bun run dev:remove        # stops and removes the test copy; its settings are kept
bun run release:install   # once published: installs the latest release if missing or older
```

`powershell -File scripts/remove-dev-app.ps1 -DeleteData` also deletes the test copy's settings.

## The signing key (one-time setup, keep it safe)

Updates are signed with a minisign key pair; the app only installs files signed by it and only
moves forward in version.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-update-signing.ps1
```

The script:

- creates the key if needed in `%USERPROFILE%\.tauri\l8toolkit-updater.key` (outside the
  repository) and asks for its password;
- writes the public key into `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`);
- once the GitHub repository exists, checks the password and stores the repository secrets
  `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

**Back up the key file and keep the password in a password manager.** Without them no future
update can be signed for the copies already installed; users would have to reinstall by hand.
The private key is never committed, logged or put in `.env`.

Until the secrets exist, the release workflow skips publishing with a warning.

## What the updater enforces

- One fixed HTTPS endpoint: `https://github.com/leeBrown71309/L8Toolkit/releases/latest/download/latest.json`.
- The installer's signature must match the embedded public key, and it must carry the version it
  was signed for (`requireSignedVersion`), so an old genuine installer cannot be passed off as new.
- Only strictly newer versions are installed: no downgrade.
- The NSIS installer runs in passive mode, per user, and restarts the app.

## Code signing (later, free for open source)

The installer is not Authenticode-signed, so SmartScreen warns on first download. SignPath
Foundation signs open-source projects for free (public repository, OSI licence: MIT is fine);
apply at <https://signpath.org/> and plug their GitHub action into the release workflow.
