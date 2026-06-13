# 2026-06-13 — Parallel Sandbox / Production modes

Continues from `2026-06-11_12-20_hmrc_connection_screen_and_stateful_test_data.md`.
A large refactor: the app now runs in two fully parallel modes — **Sandbox** and
**Production** — each with its own database schema and its own HMRC credentials,
switchable live from a topbar toggle with no restart. Four design decisions were
confirmed with the user up front (all "recommended" options).

## Run mode (separate JSON, AppState, commands)
- New `runmode.rs`: `RunMode { Sandbox, Production }`, persisted to its own file
  `MyOpenUKTaxApp.runmode.json` (default Sandbox). `schema()` → `"sandbox"`/
  `"production"`, `hmrc_environment()` → base-URL selector.
- `AppState` gained `run_mode: Mutex<RunMode>` + `current_run_mode()`.
- `commands/mode_cmds.rs`: `get_run_mode`, `set_run_mode`. `set_run_mode` persists the
  flag, flips the DB active schema, and updates AppState — all live.

## Database — two ATTACHed schemas
- `db/mod.rs` rewritten. A `:memory:` control connection `ATTACH`es two files —
  `Data/MyOpenUKTaxApp.sandbox.db` and `…production.db` — under schema aliases
  `sandbox` / `production`. `active_schema` selects which one every query targets;
  `set_active_schema` flips it instantly (both stay attached).
- Every query is schema-qualified (`{schema}.table`); DDL is generated per schema;
  migrations run **per attached file** (each carries its own `PRAGMA <schema>.user_version`);
  defaults (subcategories, HMRC categories, `app_meta`) seed into both. Backups are
  now `VACUUM <schema> INTO …` for the active schema, filenamed per mode.
- **One-time data migration:** an existing single `MyOpenUKTaxApp.db` is renamed to
  `…sandbox.db` on first open (so current data becomes the sandbox schema); production
  starts empty. Verified: sandbox.db retained the prior events/submissions, production.db
  is a fresh schema.

## Settings — per-mode HMRC credentials
- `HmrcSettings` lost its `environment` field (the mode now drives the base URL).
  `Settings.hmrc` was split into `hmrc_sandbox` + `hmrc_production`; `#[serde(alias = "hmrc")]`
  migrates an older single-block file into the sandbox slot. New `Settings::hmrc(mode)` /
  `hmrc_mut(mode)` accessors.
- Every HMRC command (`hmrc_cmds.rs`) now reads/writes the active mode's block and uses
  `mode.hmrc_environment()` for the client base URL. `update_settings` preserves the OAuth
  tokens for **both** blocks. Removed `ALLOWED_HMRC_ENVIRONMENTS` / the environment validation.

## Frontend — toggle, root class, runmode CSS, cache invalidation
- `app-store.ts`: `run_mode` + `set_run_mode`. `api.ts`/`types.ts`: `RunMode`,
  `get_run_mode`/`set_run_mode`, and the split `hmrc_sandbox`/`hmrc_production` Settings.
- `app-shell.tsx`: seeds the mode from `get_run_mode` on launch and puts
  `mode-sandbox` / `mode-production` on the root element.
- `index.css`: the run-mode visibility system — `.mode-production .runmode_sandbox` and
  `.mode-sandbox .runmode_production` are `display:none`. So `runmode_sandbox` /
  `runmode_production` elements show only in their mode; `runmode_universal` (and anything
  untagged) always shows. Toggling the root class re-skins the app with no restart.
- `topbar.tsx`: a **Sandbox/Production segmented toggle** (Production highlighted red). On
  switch it calls `set_run_mode`, resets the HMRC LED, and `invalidateQueries()` (everything)
  so all screens reload from the now-active schema.
- `hmrc-connection.tsx`: binds the credential fields to the active mode's block
  (`draft[mode_key]`), replaced the Environment select with a read-only mode indicator,
  tagged the mock-identity/scenario block and the Sandbox test-data card `runmode_sandbox`,
  and added a `runmode_production` warning banner.

## Verification
- `cargo check` + `npm run typecheck` clean at each phase.
- App relaunches cleanly; both schemas open and migrate. Data-migration confirmed by
  scanning the DB files (sandbox = data, production = empty schema).

## State at end of session
- All changes **uncommitted** (version 0.0.6); user commits manually. Dev app left running.
- New on-disk artifacts: `MyOpenUKTaxApp.runmode.json`, `Data/MyOpenUKTaxApp.{sandbox,production}.db`
  (the legacy `MyOpenUKTaxApp.db` is consumed by the migration). **DB schema change** — both
  schema files migrate automatically on launch.

## Notes / follow-ups
- The MCP server shares the same connection, so its queries follow the active schema too.
- Live runtime behaviour (toggle → data reload + UI reskin, no restart) still wants a manual
  click-through to confirm; everything compiles and launches.
- Production fraud-prevention headers remain unreviewed (flagged previously) before any real
  production submission.
