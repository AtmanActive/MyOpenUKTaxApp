# 2026-06-07 — GitHub CI/CD, multi-platform releases, and the rebuild-loop fix

Continues from `2026-06-06_13-39_initial_scaffold_and_skeleton.md`. This session
took the project from "compiles and boots locally" to "committed, building, and
releasing on GitHub", then iterated on the release pipeline.

## First-run bug fixed

- Booting the app via `tauri dev` surfaced a fatal startup error: `HmrcSettings`
  used `#[derive(Default)]`, which ignores serde field defaults, so `environment`
  was `""` and failed settings validation on first save. Replaced with a manual
  `Default` impl (sandbox + default redirect URI). App now boots, creates its
  portable settings/`Data`/`Logs`, binds the MCP server, and the UI round-trips
  to the Rust commands.

## Source control

- Committed the full skeleton and pushed to `main` (no feature branches, per the
  user's preference). Local git identity set per-repo to
  `AtmanActive <AtmanActive@users.noreply.github.com>`.
- `.gitignore` already excludes build output, the SQLite DB, the settings JSON
  (HMRC secrets) and logs; verified with `git check-ignore`.

## Release pipeline (evolved over several iterations)

1. Started Windows-only (NSIS) auto-on-push.
2. Expanded to a 3-OS build matrix — Windows, macOS, Linux — all publishing to
   one release, with a single version-bump `prepare` job feeding the matrix.
   macOS builds a universal (Apple Silicon + Intel) binary; Linux installs the
   WebKitGTK/GTK build deps.
3. Added a Windows **portable ZIP** (Tauri has no native zip target, so the
   compiled exe is zipped) alongside the NSIS installer.
4. Took control of artifact names: build without auto-publish, then rename every
   bundle with an OS/arch token and upload via `scripts/publish-artifacts.mjs`
   (reads tauri-action's `artifactPaths`); the release is created up front in
   `prepare`. Portable ZIP now nests the exe in a `MyOpenUKTaxApp/` folder.

Final artifact names: `MyOpenUKTaxApp_<v>_windows_x64_setup.exe`,
`_windows_x64_portable.zip`, `_macos_universal.dmg`, `_linux_amd64.deb`,
`_linux_amd64.AppImage`. Binaries are unsigned (documented in README).

## Rebuild-loop fix (the key change)

Auto-on-push + rust-analyzer/Tauri rewriting `Cargo.lock`'s package version
created a loop (commit → release → bump → pull → lockfile rewrite → commit …).
Fixed two ways:

- **Manual-only releases:** workflow now triggers on `workflow_dispatch` only.
  Run via **Actions → release → Run workflow** or `gh workflow run release.yml`.
- **Lockfile sync:** `scripts/bump-version.mjs` now also bumps the version inside
  `Cargo.lock`, and CI commits it, so the working tree stays consistent.

## Docs

- Added `docs/design/github_setup.md` — a terse CI/CD reference (kept brief as it
  may load via `AGENTS.md`).
- README gained a Downloads & installation table and a Signed vs unsigned
  binaries section (SmartScreen / Gatekeeper / AppImage guidance).

## Verification

- `cargo check`, `npm run build`: green. App boots; MCP listening on
  `127.0.0.1:8765`; dashboard/settings commands invoked successfully.
- Workflow YAML and both Node scripts validated (pyyaml parse + `node --check`);
  `Cargo.lock` rename regex tested against the real lockfile.

## Open items / next steps

- Cut a manual release and confirm all five artifacts land with the new names.
- Optional: a checks-only `ci.yml` (cargo check + frontend build) on push.
- Code signing (Authenticode + Apple notarization) via repo secrets.
- `docs/design/github_setup.md` is not yet committed; optionally reference it
  from `AGENTS.md`.
