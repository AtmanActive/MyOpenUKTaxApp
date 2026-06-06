# 2026-06-06 — Initial scaffold and full skeleton

Living worklog for the first build session of **MyOpenUKTaxApp**.

## Goal of this session

Stand up the whole application end-to-end as a navigable skeleton: a portable
Tauri 2 desktop app with all infrastructure real (paths, settings, logging,
SQLite + backups, theming, responsive shell), all seven sections present and
wired to a Rust backend, a real HMRC MTD client, and an embedded MCP server.

## Decisions taken (clarifying questions)

Three questions were asked up front (per `AGENTS.md`) and answered:

1. **Scope** → *Full skeleton first*: everything scaffolded, compiles, runs, and
   is navigable; depth added section-by-section later.
2. **HMRC MTD** → *Wire the sandbox now*: real OAuth authorisation-code flow,
   fraud-prevention headers and MTD endpoints, with credentials read from the
   settings JSON at runtime (never hardcoded). Live testing needs Developer Hub
   credentials supplied by the user.
3. **SQLite access** → *Rust backend layer* (rusqlite via Tauri commands), best
   fit for the exe-adjacent DB, backups/pruning, validation and logging.

## What was built

### Backend (`src-tauri/src`)
- `paths.rs` — portable exe-relative path discovery; all data under the exe dir.
- `settings.rs` — `MyOpenUKTaxApp.settings.json`, created with defaults, atomic
  save, validation; holds appearance, retention windows, MCP and HMRC config +
  a generated `device_id`.
- `logging.rs` + `housekeeping.rs` — three per-session log channels
  (Action/Debug/Network) with the specified filename format and age pruning.
- `db/` — rusqlite layer: schema migrations (PRAGMA user_version), seeds the six
  default subcategories and the fixed HMRC self-employment categories, "smart"
  debounced pre-write backups via `VACUUM INTO`, backup pruning, and all CRUD +
  dashboard/period aggregation. Money stored as integer pence.
- `hmrc/` — MTD client: OAuth authorize-url / code-exchange / refresh, fraud
  headers, `hello/world` connectivity check, self-employment period submission,
  with all requests logged to the Network channel (secrets masked).
- `commands/` — Tauri commands for subcategories, events, mappings, dashboard,
  settings and HMRC (`rename_all = "snake_case"`).
- `mcp/` — embedded MCP server (tiny_http, localhost only) exposing
  initialize / tools/list / tools/call over JSON-RPC, sharing the DB handle.
- `state.rs`, `lib.rs`, `error.rs`, `util.rs` — shared state, startup wiring,
  central error type, id/nonce helper.

### Frontend (`src/`)
- Foundation: `lib/types.ts`, `lib/api.ts` (typed `invoke` wrappers),
  `lib/format.ts`, zustand stores (`app-store`, `notify`), shadcn/ui-style
  primitives, Material Symbols icon wrapper.
- Shell: responsive sidebar (vertical, or bottom taskbar in portrait), dynamic
  topbar (search/date filters), main pane; theme provider (system/light/dark via
  `prefers-color-scheme`) and font-size scaling.
- Sections: Dashboard, Add Event (Income/Expenses tab-switch + read-only/clone),
  Events (two sortable tables + filters), Subcategory Management, Category
  Mapping, HMRC (connect flow + submit + history), Settings.

### CI/CD
- `scripts/bump-version.mjs` — single-digit-carry version bump syncing
  `version.txt`, `package.json`, `tauri.conf.json`, `Cargo.toml`.
- `.github/workflows/release.yml` — on push to main: bump, commit back, build,
  and publish a GitHub release with the installer.

## Verification

- `cargo check` (backend): **green, 0 warnings**.
- `npm run build` (tsc + vite): **green**.
- Not yet run this session: full `npm run tauri build` bundle and a live launch
  via `npm run tauri dev` (next step).

## Deviations from the coding standards (with reasons)

Per `AGENTS.md`, deviations are documented here:

- **Keyword/named arguments**: Rust has no named-argument call syntax, so Rust
  calls are positional with descriptive parameter names (and param structs where
  a call is large). TS payloads use named object fields, satisfying the intent.
- **Allman braces**: hand-written in Rust and TS. rustfmt's Allman options are
  nightly-only (configured in `rustfmt.toml`); Prettier has no Allman option, so
  no auto-formatter is run that would undo it.
- **snake_case names**: applied to all author-controlled identifiers. React
  components must be PascalCase and DOM/library props (`className`, `onClick`)
  are fixed by the framework — those remain as required.
- **`src` layout**: the frontend lives in `src/`; the Rust crate uses the
  conventional Tauri `src-tauri/` sibling. Fighting that convention would add
  fragility for no benefit.
- **shadcn/ui**: implemented shadcn-style primitives directly (same cva +
  tailwind-merge + CSS-variable token approach) instead of via the CLI, for
  deterministic, offline-safe setup. Native form controls are used for
  dropdowns/dates/numbers/checkboxes (the spec already relies on the HTML
  `title` attribute for hints).

## How to run

```
npm install
npm run tauri dev      # launch the app
cargo check --manifest-path src-tauri/Cargo.toml
npm run build          # typecheck + build frontend
```

Portable data appears next to the executable (in dev: `src-tauri/target/debug/`):
`MyOpenUKTaxApp.settings.json`, `Data/`, `Logs/`.

## Next steps / open items

- Run `tauri dev` to validate runtime behaviour and the MCP endpoint.
- HMRC: register a Developer Hub app, fill credentials in Settings, complete the
  OAuth redirect listener (currently the code is pasted manually) and verify the
  CSRF `state`.
- Flesh out the period-summary body to the exact MTD schema/version and add a
  multi-OS CI matrix and true portable packaging.
- Test-first coverage (unit/component tests) as features deepen.
