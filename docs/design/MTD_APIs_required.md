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
| Self Employment Business (MTD) | 5.0 | `POST /individuals/business/self-employment/{nino}/{businessId}/period` | user-restricted | Submit a quarterly period summary |

> **Note (Self Employment Business 5.0):** the version (Accept header) is aligned
> with the subscription, but the v5 request/response *schema* has not yet been
> verified end-to-end against the sandbox. Confirm the period-summary payload
> shape when submissions are first tested, and update `build_period_body` if needed.

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
