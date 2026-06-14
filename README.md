# MyOpenUKTaxApp

UK HMRC MTD desktop application for the self-employed.

A portable, cross-platform desktop app for simple self-employment accounting
(income & expenses) that can submit quarterly data directly to
[HMRC Making Tax Digital for Income Tax](https://www.gov.uk/government/collections/making-tax-digital-for-income-tax)
via the [HMRC API](https://developer.service.hmrc.gov.uk/api-documentation). Includes an MCP server!

## Current Status
The app is currently in heavy development. The changes are happening daily. The releases are usually a week behind.


## Stack

- **Tauri 2** desktop shell (Rust backend)
- **Vite + React + TypeScript + Tailwind CSS** (shadcn/ui-style components)
- **SQLite** (rusqlite) for storage, accessed through a Rust data layer
- Embedded **MCP server** so an AI agent can query/control the app while it runs

## Portability

The app is self-contained: it discovers its own executable directory and keeps
**all** data beside the executable — never in OS-specific folders.

```
<app folder>/
  MyOpenUKTaxApp(.exe)
  MyOpenUKTaxApp.settings.json
  Data/MyOpenUKTaxApp.db
  Data/Backups/
  Logs/Action/  Logs/Debug/  Logs/Network/
```

In development these are created under `src-tauri/target/debug/`.

## Prerequisites

- Node.js 20+ and npm
- Rust (stable, MSVC toolchain on Windows)
- Platform Tauri prerequisites — see <https://tauri.app/start/prerequisites/>
  (on Windows: Microsoft Visual Studio C++ Build Tools and the WebView2 runtime)

## Develop

```bash
npm install
npm run tauri dev        # launch the app with hot reload
```

## Verify

```bash
npm run build                                   # typecheck + build the frontend
cargo check --manifest-path src-tauri/Cargo.toml # type-check the Rust backend
```

## Build a release locally

```bash
npm run tauri build      # produces the installer under src-tauri/target/release
```

## HMRC setup

Register an application on the
[HMRC Developer Hub](https://developer.service.hmrc.gov.uk/), then enter the
client id/secret, redirect URI, your National Insurance number and MTD business
id on the in-app **Settings** screen. Credentials are stored only in the local,
git-ignored `MyOpenUKTaxApp.settings.json` — never in source control.

## Project layout

```
src/            React + TypeScript frontend (sections, components, lib, store)
src-tauri/      Rust backend (paths, settings, logging, db, hmrc, commands, mcp)
scripts/        Build/versioning helpers
docs/           Design docs and the development worklog
.github/        CI/CD release workflow
```

## Versioning & releases

`version.txt` is the source of truth. Releases are **triggered manually** — from
the repository's **Actions → release → Run workflow**, or with
`gh workflow run release.yml`. Ordinary pushes do **not** build or release, so
routine commits never bump the version. Each run bumps the version
(single-digit-per-component carry: `0.0.9 → 0.1.0`), syncs `package.json`,
`tauri.conf.json`, `Cargo.toml` and `Cargo.lock`, commits the bump, then builds on
Windows, macOS and Linux in parallel and publishes one GitHub release carrying
every platform's artifacts.

## Downloads & installation

Each [GitHub release](https://github.com/AtmanActive/MyOpenUKTaxApp/releases)
contains:

| Platform | Artifact | Notes |
|----------|----------|-------|
| Windows  | `MyOpenUKTaxApp_<version>_windows_x64_setup.exe`    | NSIS installer |
| Windows  | `MyOpenUKTaxApp_<version>_windows_x64_portable.zip` | **Portable** — unzip, open the `MyOpenUKTaxApp` folder, run `MyOpenUKTaxApp.exe` (no installation) |
| macOS    | `MyOpenUKTaxApp_<version>_macos_universal.dmg`      | Universal (Apple Silicon + Intel) |
| Linux    | `MyOpenUKTaxApp_<version>_linux_amd64.deb`          | Debian/Ubuntu package |
| Linux    | `MyOpenUKTaxApp_<version>_linux_amd64.AppImage`     | Portable Linux binary (`chmod +x` then run) |

### Signed vs unsigned binaries

These releases are currently **unsigned** — the project does not yet have code
-signing certificates. The software is safe, but because it is not signed by a
recognised certificate authority, the operating system will show a warning the
first time you run it. This is expected; here is how to proceed on each platform:

- **Windows** (installer and portable `.exe`) — Microsoft Defender SmartScreen
  shows *“Windows protected your PC.”* Click **More info → Run anyway**.
- **macOS** (`.dmg`) — the app is unsigned and un-notarized, so Gatekeeper says
  the developer *“cannot be verified”* (or that the app *“is damaged”*).
  Right-click the app and choose **Open** the first time, or clear the quarantine
  flag from Terminal:
  ```bash
  xattr -dr com.apple.quarantine /Applications/MyOpenUKTaxApp.app
  ```
- **Linux** (`.deb` / `.AppImage`) — Linux does not gate unsigned desktop apps the
  same way. For the AppImage, just make it executable: `chmod +x MyOpenUKTaxApp_*.AppImage`.

**Planned:** proper signing will remove these warnings — Authenticode signing on
Windows and Apple Developer signing + notarization on macOS — once the
certificates are available. They will be wired into CI as encrypted repository
secrets (no private keys in the repo).

## License

MIT — see [LICENSE](LICENSE).
