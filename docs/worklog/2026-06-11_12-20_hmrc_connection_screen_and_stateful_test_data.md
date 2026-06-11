# 2026-06-11 — "HMRC Connection" screen, stateful sandbox test data, LED

Continues from `2026-06-11_11-30_hmrc_get_screen.md`. Motivation: the HMRC Get
screen showed HMRC's *canned* sandbox stubs (obligations dated 2018–2020) and never
reflected our own submission, because the sandbox is stateless unless you use the
`STATEFUL` Gov-Test-Scenario against a test user that owns a business. This session
makes the sandbox dots connect and reorganises all HMRC connection concerns onto a
dedicated screen.

## Why the old screen showed someone else's data (diagnosis)
The HMRC sandbox returns fixed example payloads by default; only the `STATEFUL`
scenario persists what you submit and reads it back, and only for a test user that
has a business. There is **no** "create business" in the real API — businesses are
seeded via the sandbox-only **MTD Self Assessment Test Support API**
(`POST /individuals/self-assessment-test-support/business/{nino}`), data auto-purged
after 7 days. ([Test users, test data and stateful behaviour](https://developer.service.hmrc.gov.uk/api-documentation/docs/testing/test-users-test-data-stateful-behaviour))

## Gov-Test-Scenario: back to client-level, default STATEFUL (user-approved)
Reversed the earlier "scenario only on the business lookup" scoping. The scenario is
once again a **client-wide** header (`HmrcClient::new(env, logger, Option<String>)`,
injected in `send`), sent on every API call when sandbox + "using mock identity" +
non-empty. Default scenario changed from `BUSINESS_AND_PROPERTY` to **`STATEFUL`**
(the value that makes submit → obligations → retrieve line up). Command layer gained
`effective_scenario(env, using_mock_identity, scenario)`; token/connectivity calls
pass `None`, data calls (list/submit/all GETs) pass the effective scenario.

## Sandbox test-data seeding (idempotent) — the "(B)" helper
- `hmrc/mod.rs`: `create_test_business` + `create_test_itsa_status`
  (`TEST_SUPPORT_ACCEPT` = 1.0).
- `hmrc_setup_test_data(tax_year)` command: lists businesses with a **STATEFUL**
  client → reuses an existing self-employment business, else creates one; persists
  the `businessId`; then sets ITSA status "MTD Mandated" for the year. Re-running
  reuses the business (no duplicate) — idempotent, as requested. Sandbox-only.
- **Bug found + fixed during testing:** the test-support *seeding* POSTs reject a
  Gov-Test-Scenario (`RULE_INCORRECT_GOV_TEST_SCENARIO`). Fix: two clients in the
  command — a STATEFUL client for the read (list businesses), and a **scenario-less**
  client for the create-business / ITSA-status seeding calls.

## New "HMRC Connection" screen + sidebar LED
- New `SectionId` `hmrc-connection` and sidebar item (icon `key`), placed before
  HMRC Put. `app-shell.tsx` renders `HmrcConnectionSection`.
- `src/sections/hmrc-connection.tsx` (new) consolidates:
  - **Credentials** (moved from Settings): environment, client id/secret, NINO,
    Business ID, "Using mock identity", scenario (placeholder now `STATEFUL`),
    redirect URIs, "Fetch my businesses" + picker. Same debounced auto-save pattern.
  - **Connection** (moved from HMRC Put): status badges + Test / Authorise / Refresh.
  - **Sandbox test data** (sandbox only): the "Set up test data" button + result.
- **Settings** lost its HMRC card (and the now-unused NINO helpers / TextField /
  `push`). **HMRC Put** is now just the submit form + submission history.
- **Sidebar LED** right of the "HMRC Connection" label, four states via a new store
  field `hmrc_connection` (`unknown`/`connecting`/`connected`/`failed`): grey, cyan
  (pulsing) while signing in, green, red. Seeded green from `hmrc_status` on launch
  (token present), then driven by the Authorise/Test/Refresh mutations.

## 7-day expiry tracking (this request)
- DB migration **v1 → v2**: a small `app_meta(key, value)` table with `get_meta` /
  `set_meta`.
- `hmrc_setup_test_data` records `test_data_seeded_at` (RFC3339) on every successful
  run; new `hmrc_test_data_seeded_at` command reads it.
- HMRC Connection sandbox card shows **"Test data last set up"**, **"Expires around"**
  (+7 days), and a short explanation that HMRC deletes sandbox data after 7 days and
  to re-seed when it expires. Persists across restarts (it's in the DB); refreshes
  immediately after each setup click.

## Files
- Backend: `hmrc/mod.rs`, `commands/hmrc_cmds.rs`, `commands/settings_cmds.rs` (read
  only), `db/mod.rs`, `lib.rs`, `settings.rs` (default scenario → STATEFUL).
- Frontend: `sections/hmrc-connection.tsx` (new), `sections/settings.tsx`,
  `sections/hmrc-history.tsx`, `components/layout/{sidebar,app-shell}.tsx`,
  `store/app-store.ts`, `lib/api.ts`, `lib/types.ts`.

## Verification (sandbox)
- `cargo check` + `npm run typecheck` clean throughout.
- "Set up test data" verified end-to-end: first run creates a business
  (`X6IS12961443758`), re-run reuses it (idempotent). Network log confirmed the
  STATEFUL list read (200) and the scenario-less seeding POSTs after the fix.
- Seeded-at + expiry box confirmed showing in the UI and persisting.

## State at end of session
- All changes **uncommitted** (version 0.0.6); the user commits manually. Dev app
  left running.
- The sandbox now connects the dots: set up test data → submit on HMRC Put → see it
  reflected on HMRC Get (with `STATEFUL`).

## Notes / possible follow-ups
- Production fraud-prevention headers still need review before a real-account run.
- HMRC validation errors are still only visible via the submission's response JSON
  (surfacing them inline remains a nice-to-have).
