# HMRC MTD APIs required

The HMRC APIs (and exact versions) this app calls. On the HMRC Developer Hub you
must **add/subscribe your application to each of these, at the listed version**,
in the environment you use (Sandbox and/or Production). If the app is subscribed
to a *different* version than the code requests, calls fail with HTTP 404
`MATCHING_RESOURCE_NOT_FOUND` ("a resource with the name in the request can not
be found in the API").

The versions below are the `Accept: application/vnd.hmrc.<version>+json` values
sent by the code; they are defined as constants in
[`src-tauri/src/hmrc/mod.rs`](../../src-tauri/src/hmrc/mod.rs) (`*_ACCEPT`).
**Keep this table in sync with those constants** whenever an endpoint is added
or a version is bumped.

## APIs the code calls

### Required (Phase 1) — must be subscribed for the app to work

| HMRC Developer Hub API | Version | Endpoint(s) used | Screen | Purpose |
|------------------------|---------|------------------|--------|---------|
| Hello World | 1.0 | `GET /hello/world` | HMRC Put | Connectivity test ("Test connection") |
| Business Details (MTD) | 2.0 | `GET /individuals/business/details/{nino}/list` · `GET .../details/{nino}/{businessId}` | HMRC Put / Get | List businesses (pick Business ID) · retrieve one business's details |
| Self Employment Business (MTD) | 5.0 | `PUT .../self-employment/{nino}/{businessId}/cumulative/{taxYear}` (submit) · `GET .../cumulative/{taxYear}` · `GET .../annual/{taxYear}` · `GET .../period/{taxYear}` | HMRC Put / Get | Submit + read back the cumulative summary, annual submission, legacy period summaries |
| Obligations (MTD) | 3.0 | `GET /obligations/details/{nino}/income-and-expenditure` · `GET .../crystallisation` | HMRC Get | Quarterly + final-declaration obligation status (open/fulfilled) |

> **Note (Self Employment Business 5.0 — cumulative model):** from tax year
> **2025-26 onwards** HMRC replaced the per-quarter "Create Period Summary"
> (`POST .../period`, which only accepts tax years **2024-25 and earlier**) with a
> **cumulative** model: each submission sends the running totals from the start of
> the tax year (6 April) up to the chosen end date, via
> `PUT .../cumulative/{taxYear}` (taxYear formatted `YYYY-YY`, e.g. `2025-26`).
> The request body keeps the same shape — `periodDates` (`periodStartDate` =
> 6 April, `periodEndDate`), optional `periodIncome`, optional `periodExpenses`
> (HMRC category codes → amounts). Built by `build_cumulative_body` and sent by
> `submit_cumulative_period`. Submitting a date in an unsupported tax year (e.g. a
> current-year date to the old `period` endpoint) returns HTTP 400
> `RULE_TAX_YEAR_NOT_SUPPORTED`. The legacy `GET .../period/{taxYear}` likewise
> only returns data for ≤2024-25.

### Optional (Phase 2) — "HMRC Get" cards that 404 until subscribed

| HMRC Developer Hub API | Version | Endpoint used | Purpose |
|------------------------|---------|---------------|---------|
| Business Income Source Summary (MTD) | 3.0 | `GET /individuals/self-assessment/income-summary/{nino}/{typeOfBusiness}/{taxYear}/{businessId}` | HMRC's computed income/expense summary per business |
| Individual Calculations (MTD) | 8.0 | `GET /individuals/calculations/{nino}/self-assessment/{taxYear}` | List of tax calculations for the year |
| Self Assessment Accounts (MTD) | 4.0 | `GET /accounts/self-assessment/{nino}/balance-and-transactions?onlyOpenItems=true` | Open balance, charges and payments |

> **Sandbox note (Gov-Test-Scenario):** the scenario header is **only** sent on the
> Business Details *List Businesses* call, and only when Settings has Sandbox +
> "Using mock identity" on with a non-empty scenario. It is **not** sent on
> submissions or on any "HMRC Get" read. If a sandbox GET needs a scenario to
> return stub data, add per-endpoint scenario support (not currently implemented).

> **Version constants:** the `*_ACCEPT` constants in
> [`src-tauri/src/hmrc/mod.rs`](../../src-tauri/src/hmrc/mod.rs) are the single
> source of truth for the versions above — keep this table in sync with them.

## OAuth (authorisation-code flow)

Not an API you "add", but required for the user-restricted calls above:

- Authorise: `GET {base}/oauth/authorize`
- Token / refresh: `POST {base}/oauth/token`
- Scopes: `read:self-assessment write:self-assessment`
- Redirect URIs (loopback) registered on the Developer Hub — see Settings → HMRC
  for the exact list (e.g. `http://localhost:8350/oauth/callback` … `8354`).

## Hosts

- Sandbox: `https://test-api.service.hmrc.gov.uk`
- Production: `https://api.service.hmrc.gov.uk`

## Recommended for sandbox testing (not called by the app)

- **Create Test User 1.0** — to mint a test user (NINO + MTD ITSA enrolment).
- Many MTD endpoints return stubbed data selected via the `Gov-Test-Scenario`
  request header (configurable in Settings → HMRC when Environment = sandbox).
  For *Business Details → List All Businesses*, `BUSINESS_AND_PROPERTY` returns a
  self-employment + property set; `STATEFUL` reads back data created in the
  stateful sandbox.
