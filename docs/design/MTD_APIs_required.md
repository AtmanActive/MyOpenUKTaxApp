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

| HMRC Developer Hub API | Version | Endpoint(s) used | Auth | Purpose |
|------------------------|---------|------------------|------|---------|
| Hello World | 1.0 | `GET /hello/world` | none (open) | Connectivity test ("Test connection") |
| Business Details (MTD) | 2.0 | `GET /individuals/business/details/{nino}/list` | user-restricted | List businesses → pick/auto-fill the Business ID |
| Self Employment Business (MTD) | 5.0 | `PUT /individuals/business/self-employment/{nino}/{businessId}/cumulative/{taxYear}` | user-restricted | Submit a cumulative (year-to-date) period summary |

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
> `RULE_TAX_YEAR_NOT_SUPPORTED`.

> **Sandbox note:** the `Gov-Test-Scenario` set in Settings is sent on *all* API
> calls. A scenario chosen for the business lookup (e.g. `BUSINESS_AND_PROPERTY`)
> is **not** valid for the cumulative submission — clear the field (or pick a
> submission-appropriate scenario) to get the default success response when
> testing a submit.

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
