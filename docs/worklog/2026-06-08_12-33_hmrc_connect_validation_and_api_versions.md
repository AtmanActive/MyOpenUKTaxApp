# 2026-06-08 — HMRC connect polish, NINO validation, sandbox scenarios, and API-version fix

Continues from `2026-06-07_18-21_settings_ux_updates_about_and_hmrc_oauth.md`. Focus
this session: making the HMRC connect flow friendly and correct, and getting the
*List Businesses* call actually working. App ran under `npm run tauri dev`
throughout; every change verified with `cargo check` / `npm run typecheck`.

## Connect-flow UX

- **Window raises itself on callback.** The moment the OAuth redirect lands on the
  loopback listener, `hmrc_authorize` calls `unminimize()/show()/set_focus()` on
  the main window (new `bring_window_to_front`), so the app comes to the front and
  the user sees the result without dismissing the browser.
- **Auto-validate + auto-prefill after Authorise.** On the HMRC screen, a
  successful authorise now triggers a read-only *List Businesses* call that
  doubles as the NINO↔account authorisation check and pre-fills the Business ID:
  - exactly one business → it is saved automatically (new `hmrc_set_business_id`
    command) and "Connected — using your business (…)".
  - several → "found N — pick one on Settings".
  - none → informative message.
  The Authorise button shows "Waiting for sign-in…" then "Checking access…".

## NINO handling

- Backend `normalize_nino` (upper-case, strip spaces) + `validate_nino` (shape
  `AB123456C`) run before the network call, so typos fail instantly.
- `hmrc_list_businesses` now maps HTTP status to clear messages: 400 (NINO
  rejected), 401 (sign-in expired), 403 (account not authorised for that NINO —
  the user-confusion case), 404, other.
- Settings NINO field auto-normalises as you type and shows a live red hint until
  the shape is valid.

## Sandbox testing aids

- **`Gov-Test-Scenario` support.** New sandbox-only settings field; when set, the
  value is sent as the `Gov-Test-Scenario` header on API calls (sandbox only).
  Settings shows the field only when Environment = sandbox.
- **HMRC error-body logging.** `HmrcClient::send` now logs a snippet of any
  non-2xx response body to the Network channel (no secrets in error bodies),
  which is what let us diagnose the 404 below.

## The 404 investigation and root cause

- *Fetch my businesses* kept returning HTTP **404**. Logs (via the new body
  logging) showed `MATCHING_RESOURCE_NOT_FOUND` — HMRC's gateway "this
  endpoint/version isn't available to your app", **not** "no data".
- The user's Developer Hub application-details PDF revealed the cause: the app is
  subscribed to **Business Details (MTD) 2.0**, but the code requested **1.0**.
  Requesting a version the app isn't subscribed to yields exactly that 404.
- **Fix:** the List Businesses call now requests **v2.0**.

## API versions bumped + centralised

- All HMRC API versions are now named constants in `hmrc/mod.rs`:
  `HELLO_WORLD_ACCEPT` = 1.0, `BUSINESS_DETAILS_ACCEPT` = 2.0,
  `SELF_EMPLOYMENT_BUSINESS_ACCEPT` = **5.0** (bumped from 2.0 to match the
  subscription, for quarterly submissions).
- New living doc **`docs/design/MTD_APIs_required.md`** lists every HMRC API the
  code calls with its version/endpoint/auth/purpose, OAuth scopes, hosts, and
  sandbox-testing notes — so the next person knows exactly what to subscribe to
  on the Developer Hub. It must be kept in sync with the `*_ACCEPT` constants.

## Clarifications given (no code)

- **Business ID is retrievable; NINO is not** — NINO is the addressing key (input
  to List Businesses), and there's no MTD endpoint that returns the signed-in
  user's own NINO.
- **NINO ↔ sign-in mismatch is safe**: the NINO is only an address; HMRC checks
  the token is authorised for it and returns 403 otherwise — no data leak.
- **"Application ID" on the Developer Hub = the OAuth Client ID** (goes in the
  Client ID field).

## State at end of session

- All changes are **uncommitted** working-tree edits (version still 0.0.5); the
  user commits to GitHub manually. Releases remain manual (`workflow_dispatch`).
- The version fix (Business Details → 2.0) is in but **not yet confirmed by a
  successful fetch** — the session ended right after the change.

## Open items / next steps

- **Verify** *Fetch my businesses* now returns data with Business Details v2.0
  (sandbox: `Gov-Test-Scenario = BUSINESS_AND_PROPERTY`, or `STATEFUL`).
- **Self Employment Business v5.0**: Accept header is aligned, but the v5
  period-summary request/response *schema* still needs verifying against the
  sandbox; update `build_period_body` accordingly when testing submissions.
- Then: end-to-end quarterly submission against the sandbox.
