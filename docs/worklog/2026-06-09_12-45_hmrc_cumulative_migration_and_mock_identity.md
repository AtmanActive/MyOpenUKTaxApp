# 2026-06-09 — HMRC: cumulative submission migration + Gov-Test-Scenario scoping

Continues from `2026-06-09_11-40_window_state_persistence.md`. Back on the HMRC
MTD flow. The authorise → fetch-business-id flow now works; this session fixed the
record submission, which was failing, and tidied the sandbox test-scenario plumbing
it exposed.

## Diagnosis: why the submission failed

The Network log (which now captures HMRC error bodies) showed the ground truth:

```
PUT? no — POST .../self-employment/EB865410A/XBIS12345678901/period -> 400
{"code":"RULE_TAX_YEAR_NOT_SUPPORTED","message":"The tax year specified does not lie within the supported range"}
```

The submitted period was `2026-06-01 .. 2026-07-01` (tax year **2026-27**). The app
used HMRC's **"Create Period Summary"** endpoint `POST .../period`, which — per the
v5.0 OpenAPI spec — only accepts **tax year 2024-25 and earlier**:

> "This endpoint can only be used for submissions for tax year 2024-25 or earlier.
> New endpoints which support cumulative submission will be provided for tax year
> 2025-26 onwards."

So it was an **endpoint** problem, not a date problem: OAuth, business ID, version
(v5.0) and body shape were all fine — HMRC parsed the dates and rejected the tax year.

## Migration to the cumulative endpoint (user-approved)

From **2025-26 onwards** HMRC replaced per-quarter period summaries with a
**cumulative** model: each submission sends running totals from the tax-year start
(6 April) to the chosen end date, via
`PUT /individuals/business/self-employment/{nino}/{businessId}/cumulative/{taxYear}`
(taxYear `YYYY-YY`). Body shape is unchanged (`periodDates`, `periodIncome`,
`periodExpenses`).

Changes:

- **`hmrc/mod.rs`**: replaced `submit_self_employment_period` (POST `.../period`)
  with **`submit_cumulative_period`** → `PUT .../cumulative/{taxYear}` (still the
  v5.0 Accept header).
- **`commands/hmrc_cmds.rs`**: `hmrc_submit_period` now takes a single **`period_end`**
  date; a new `uk_tax_year` helper derives the tax year and its 6 April start, totals
  are aggregated **year-to-date** over `[6 April .. period_end]`, and the body is built
  by the new `build_cumulative_body`. Dropped the non-standard `_unmappedEventCount`
  field from the HMRC body (unknown fields risk rejection — the unmapped count is now
  written to the action log); empty `periodIncome`/`periodExpenses` objects are omitted.
- **Frontend** (`api.ts`, `hmrc-history.tsx`): the submit card is now
  "Submit cumulative period (year-to-date)" with a single "Reporting up to" date and a
  derived tax-year / 6 April-start display. The API call takes only `period_end`.

### Verified end-to-end

Re-submitted with a **2026-27** date (the same year that had failed on the legacy
endpoint):

```
PUT .../self-employment/EB865410A/XBIS12345678901/cumulative/2026-27 -> 204
```

HTTP **204 No Content** = accepted. (The fetched doc model guessed 200-with-`periodId`;
it is actually 204 with no body — our status check uses a 2xx range, so it records
"submitted" with an empty reference. No code change needed.)

## Gov-Test-Scenario scoping + "Using mock identity"

The diagnosis surfaced a design flaw: the `Gov-Test-Scenario` header (set for the
sandbox business lookup, e.g. `BUSINESS_AND_PROPERTY`) was sent on **every** call,
including the submission — the user had to clear it before submitting. Refactored so
it is a **per-call** concern, and added an escape hatch:

- **`hmrc/mod.rs`**: removed the client-wide `is_sandbox`/`gov_test_scenario` state and
  its blanket header injection. `send` now takes `gov_test_scenario: Option<&str>`;
  **only `list_businesses`** forwards a scenario — `hello_world`, the token calls and
  `submit_cumulative_period` pass `None`.
- **`settings.rs`**: new sandbox-only `using_mock_identity: bool` (defaults **true**).
  `commands/hmrc_cmds.rs` (`hmrc_list_businesses`) sends the scenario header only when
  `sandbox && using_mock_identity && scenario non-empty`; otherwise nothing — which is
  the escape hatch for testing against a real sandbox identity.
- **Settings UI** (`settings.tsx`, `types.ts`): a "Using mock identity" checkbox shows
  under Sandbox; the scenario field is nested beneath it (only shown when ticked) and
  its help text now says it is sent on the business lookup, not on submissions.

### Default scenario value (user request)

- **`settings.rs`**: `gov_test_scenario` now defaults to **`BUSINESS_AND_PROPERTY`**
  (new `default_gov_test_scenario()`), so fresh installs are pre-populated.
- **`settings.tsx`**: the empty-field placeholder now ends with `(BUSINESS_AND_PROPERTY)`
  as a hint.
- One-off: set the running dev settings file's empty `gov_test_scenario` back to
  `BUSINESS_AND_PROPERTY` (and added `using_mock_identity: true`) so it shows pre-filled
  without retyping. (Settings live in `src-tauri/target/debug/` under `tauri dev`.)

## Docs

- **`docs/design/MTD_APIs_required.md`** updated: Self Employment Business row now lists
  the cumulative `PUT .../cumulative/{taxYear}` endpoint, the legacy-vs-cumulative tax
  year split, the `RULE_TAX_YEAR_NOT_SUPPORTED` cause, and a note that the
  `Gov-Test-Scenario` header is scoped to the business lookup.

## Verification

- `cargo check` and `npm run typecheck` both clean throughout.
- Cumulative submission confirmed against the sandbox (204).
- App relaunched cleanly after each change.

## State at end of session

- All changes are **uncommitted** working-tree edits (version 0.0.6); the user commits
  to GitHub manually. Dev app left running.
- The HMRC happy path now works end-to-end against the sandbox: authorise → fetch
  business id → submit cumulative period.
- Possible next steps: surface HMRC validation errors more prominently in the UI (they
  are currently only visible by expanding the submission's response JSON); consider
  retrieving/displaying the stored cumulative summary; production-environment dry run.
