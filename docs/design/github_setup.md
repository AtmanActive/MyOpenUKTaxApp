# GitHub setup

Reference for this repo's CI/CD. Keep edits terse.

- **Remote:** `https://github.com/AtmanActive/MyOpenUKTaxApp.git` (branch `main`, no feature branches — commit to `main`).
- **Local git identity:** `AtmanActive <AtmanActive@users.noreply.github.com>` (set per-repo).
- **Commit trailer:** end commit messages with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.

## Releases — manual only

Workflow `.github/workflows/release.yml` triggers on **`workflow_dispatch` only** (never on push), so ordinary commits do not build, release, or bump the version. Trigger a release:

- UI: **Actions → release → Run workflow → `main`**.
- CLI: `gh workflow run release.yml`.

A run: bumps version → commits the bump to `main` → builds Windows/macOS/Linux in parallel → publishes one GitHub release with all platforms' artifacts. The bump commit's push cannot loop (trigger is manual only).

## Versioning

- `version.txt` is the source of truth; single-digit carry (`0.0.9 → 0.1.0`, `0.9.9 → 1.0.0`).
- `scripts/bump-version.mjs` bumps it and syncs `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (lockfile sync prevents rust-analyzer re-dirtying it → no rebuild loop).

## Release artifacts

`scripts/publish-artifacts.mjs` renames tauri-action bundles with an OS/arch token, then `gh release upload`s them:

- `MyOpenUKTaxApp_<v>_windows_x64_setup.exe` (NSIS installer)
- `MyOpenUKTaxApp_<v>_windows_x64_portable.zip` (portable; contains `MyOpenUKTaxApp/MyOpenUKTaxApp.exe`)
- `MyOpenUKTaxApp_<v>_macos_universal.dmg` (Apple Silicon + Intel)
- `MyOpenUKTaxApp_<v>_linux_amd64.deb`, `MyOpenUKTaxApp_<v>_linux_amd64.AppImage`

Binaries are **unsigned** (SmartScreen/Gatekeeper warnings expected; see README). Signing is a future step via repo secrets.

## Local build / verify commands

```bash
npm install                                       # install deps (or `npm ci`)
npm run tauri dev                                 # run the app (debug)
npm run build                                     # typecheck + build frontend
cargo check --manifest-path src-tauri/Cargo.toml  # type-check Rust backend
npm run tauri build                               # local release build (installer only)
node scripts/bump-version.mjs                     # bump version locally (normally CI-only)
```
