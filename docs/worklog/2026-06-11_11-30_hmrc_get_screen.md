# 2026-06-11 — "HMRC Get" screen: live, read-only state from HMRC

Continues from `2026-06-09_12-45_hmrc_cumulative_migration_and_mock_identity.md`.
Original design intent: the app should be able to show HMRC's *authoritative*
state (live, API-retrieved), so HMRC remains the source of truth even if the app
lost all local logs/data. MTD is built for exactly this — the record lives at HMRC
and is re-readable via GET endpoints. This session adds a dedicated screen for it.

## Navigation

- Renamed the existing HMRC screen **"HMRC" → "HMRC Put"** (left unchanged
  otherwise) and added a new **"HMRC Get"** screen + sidebar item
  (`cloud_download`). Both flow through `SECTIONS`, so the topbar/sidebar update
  automatically.
- `app-store.ts` (`SectionId` + `SECTIONS`), `app-shell.tsx` (`render_section`
  case + import).

## Design

HMRC is the source of truth; each card does a live GET and shows **both** a
friendly view **and** the raw JSON (collapsible), so nothing HMRC returns is
hidden. Commands return the raw `HmrcApiResult { status, body }` and do **not**
turn a non-2xx into an error, so each card surfaces HMRC's HTTP status directly —
including a 404 for an API the application is not subscribed to.

## Backend

- `hmrc/mod.rs`: new `OBLIGATIONS_ACCEPT` (3.0) and Phase-2 `pub` Accept constants
  (`BISS_ACCEPT` 3.0, `CALCULATIONS_ACCEPT` 8.0, `SA_ACCOUNTS_ACCEPT` 4.0). New GET
  client methods: `retrieve_business_details`, `get_obligations_quarterly`,
  `get_obligations_final_declaration`, `get_cumulative`, `get_annual`,
  `list_period_summaries`, plus a generic `get_raw(path, accept_version)` for the
  Phase-2 APIs. Added a `with_query` helper for URL query strings.
- `commands/hmrc_cmds.rs`: a shared `hmrc_read_context` (snapshots + validates
  token/NINO/Business ID out of the settings lock) backing nine `hmrc_get_*`
  commands, each returning `HmrcApiResult`.
- `lib.rs`: registered all nine commands.

## Frontend

- New `src/sections/hmrc-get.tsx`: a top card (connection status, **tax-year
  selector**, **Refresh all**) and one `ResultCard` per endpoint with its own
  refresh, an HTTP-status pill (404 called out as "not found / not subscribed"),
  an optional friendly renderer, and collapsed raw JSON. Queries are
  `@tanstack/react-query`, `enabled` only once a token is present, `retry: false`,
  and keyed by tax year where relevant so changing the year re-reads.
- Friendly renderers: `ObligationsTable` (tolerates both the grouped
  income-and-expenditure shape and the flat crystallisation shape; Open/Fulfilled
  badge) and `CumulativeView` (period dates + income/expense amount tables).
- Lifted `JsonBlock` out of `hmrc-history.tsx` into shared
  `src/components/ui/json-block.tsx` (now takes `data: unknown`, pretty-prints
  objects, passes strings through); `hmrc-history.tsx` updated to use it.
- `api.ts`: nine `hmrc_get_*` functions.

## Endpoints

**Phase 1 (already subscribed — verified 200 against the sandbox):**
- Business Details 2.0 — `GET .../details/{nino}/{businessId}`
- Obligations 3.0 — `GET /obligations/details/{nino}/income-and-expenditure`,
  `GET .../crystallisation`
- Self Employment Business 5.0 — `GET .../cumulative/{taxYear}`,
  `GET .../annual/{taxYear}`, `GET .../period/{taxYear}`

**Phase 2 (need their own Developer Hub subscription; cards show HTTP 404 until
enabled):**
- Business Income Source Summary 3.0 —
  `GET /individuals/self-assessment/income-summary/{nino}/{typeOfBusiness}/{taxYear}/{businessId}`
- Individual Calculations 8.0 — `GET /individuals/calculations/{nino}/self-assessment/{taxYear}`
- Self Assessment Accounts 4.0 —
  `GET /accounts/self-assessment/{nino}/balance-and-transactions?onlyOpenItems=true`
  (`onlyOpenItems=true` avoids the date-range requirement)

## Verification (sandbox, after a token refresh)

Network log showed **200** for business details, quarterly obligations,
final-declaration obligations, cumulative and annual; `period/{taxYear}` returns
**400** for 2025-26/2026-27 (expected — legacy endpoint is ≤2024-25; the card now
carries a note to that effect). Backend `cargo check` and `npm run typecheck`
clean throughout; app relaunched cleanly.

## Notes / possible follow-ups

- **No `Gov-Test-Scenario` on GETs** (it is scoped to the business lookup since the
  previous session). A sandbox card could be empty/404 even when subscribed if HMRC
  needs a scenario to return stub data → add per-endpoint sandbox scenarios if it
  bites.
- Phase-2 cards are always visible and self-document via the 404 pill; could be
  hidden behind a toggle later.
- `MTD_APIs_required.md` updated: Required (Phase 1) vs Optional (Phase 2) tables
  with every endpoint/version, and the corrected scenario-scoping note.

## State at end of session

- All changes are **uncommitted** working-tree edits (version 0.0.6); the user
  commits to GitHub manually. Dev app left running.
- "HMRC Get" gives a live, source-of-truth view that survives total local data
  loss: authorise → fetch business id → read state.
