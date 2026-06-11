// HMRC Making-Tax-Digital API client.
//
// Implements the user-restricted OAuth 2.0 authorisation-code flow and the
// authenticated calls the app needs against either the HMRC sandbox or
// production host. Credentials (client id/secret, tokens, NINO, business id)
// are supplied at runtime from settings and are never hardcoded. Every request
// and response status is written to the Network log channel, with secrets
// masked.

pub mod fraud_headers;

use crate::error::AppError;
use crate::error::AppResult;
use crate::logging::Logger;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

// HMRC hostnames per environment.
const SANDBOX_BASE_URL: &str = "https://test-api.service.hmrc.gov.uk";
const PRODUCTION_BASE_URL: &str = "https://api.service.hmrc.gov.uk";

// OAuth scopes required for MTD Income Tax Self Assessment.
const OAUTH_SCOPE: &str = "read:self-assessment write:self-assessment";

// HMRC API versions, expressed as Accept header values. These must match the
// versions the application is subscribed to on the HMRC Developer Hub. Keep them
// in sync with docs/design/MTD_APIs_required.md.
const HELLO_WORLD_ACCEPT: &str = "application/vnd.hmrc.1.0+json";
const BUSINESS_DETAILS_ACCEPT: &str = "application/vnd.hmrc.2.0+json";
const SELF_EMPLOYMENT_BUSINESS_ACCEPT: &str = "application/vnd.hmrc.5.0+json";
const OBLIGATIONS_ACCEPT: &str = "application/vnd.hmrc.3.0+json";
// Accept headers for the optional "HMRC Get" APIs (Phase 2), passed by the command
// layer to `get_raw`. The application must be subscribed to each on the Developer
// Hub for these to return data; otherwise HMRC replies 404.
pub const BISS_ACCEPT: &str = "application/vnd.hmrc.3.0+json";
pub const CALCULATIONS_ACCEPT: &str = "application/vnd.hmrc.8.0+json";
pub const SA_ACCOUNTS_ACCEPT: &str = "application/vnd.hmrc.4.0+json";
// MTD Self Assessment Test Support API — sandbox only, for seeding stateful test
// data (create a test business, set ITSA status).
const TEST_SUPPORT_ACCEPT: &str = "application/vnd.hmrc.1.0+json";

// Fixed path the local OAuth redirect listener serves.
pub const OAUTH_REDIRECT_PATH: &str = "/oauth/callback";

// Build the loopback redirect URI for a port. We use "localhost" (not 127.0.0.1)
// because the HMRC Developer Hub rejects raw-IP redirect URIs; the listener is
// bound to the IPv4 loopback (127.0.0.1) and browsers resolve/fall back
// "localhost" to it, so the redirect still lands on the listener.
pub fn redirect_uri_for_port(port: u16) -> String
{
	format!("http://localhost:{port}{OAUTH_REDIRECT_PATH}")
}

// Append a URL-encoded query string to a path. Returns the path unchanged when
// there are no parameters, so endpoints without query options stay clean.
fn with_query(path: &str, params: &[(&str, &str)]) -> String
{
	if params.is_empty()
	{
		return path.to_string();
	}
	let query = params
		.iter()
		.map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
		.collect::<Vec<_>>()
		.join("&");
	format!("{path}?{query}")
}

// The result of an HMRC API call: HTTP status plus the parsed JSON body (or a
// string wrapper if the body was not valid JSON).
#[derive(Debug, Clone, Serialize)]
pub struct HmrcApiResult
{
	pub status: u16,
	pub body: serde_json::Value,
}

// The token payload returned by the OAuth token endpoint. `scope` and
// `token_type` are captured for completeness/logging even though the app does
// not currently branch on them.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TokenResponse
{
	pub access_token: String,
	#[serde(default)]
	pub refresh_token: String,
	#[serde(default)]
	pub expires_in: i64,
	#[serde(default)]
	pub scope: String,
	#[serde(default)]
	pub token_type: String,
}

pub struct HmrcClient
{
	http_client: reqwest::Client,
	base_url: String,
	logger: Arc<Logger>,
	// When Some, sent as the Gov-Test-Scenario header on every API call (sandbox
	// stubbing / stateful testing, e.g. STATEFUL). None outside the sandbox or when
	// mock identity is off. The OAuth token endpoints never use it.
	gov_test_scenario: Option<String>,
}

impl HmrcClient
{
	// Construct a client for the given environment ("sandbox" | "production").
	// `gov_test_scenario`, when Some, is sent as the Gov-Test-Scenario header on
	// every API call (e.g. STATEFUL for stateful sandbox testing).
	pub fn new(environment: &str, logger: Arc<Logger>, gov_test_scenario: Option<String>) -> Self
	{
		let is_sandbox = environment != "production";
		let base_url = if is_sandbox { SANDBOX_BASE_URL } else { PRODUCTION_BASE_URL }.to_string();

		Self {
			http_client: reqwest::Client::new(),
			base_url,
			logger,
			gov_test_scenario,
		}
	}

	// Build the URL the user must visit to authorise the app. The caller opens
	// this in the system browser; HMRC redirects back to `redirect_uri` with a
	// `code` query parameter and the `state` echoed for CSRF protection.
	pub fn authorize_url(&self, client_id: &str, redirect_uri: &str, state: &str) -> String
	{
		format!(
			"{}/oauth/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}",
			self.base_url,
			urlencoding::encode(client_id),
			urlencoding::encode(OAUTH_SCOPE),
			urlencoding::encode(redirect_uri),
			urlencoding::encode(state),
		)
	}

	// Exchange an authorisation code for access/refresh tokens.
	pub async fn exchange_code(
		&self,
		client_id: &str,
		client_secret: &str,
		redirect_uri: &str,
		code: &str,
	) -> AppResult<TokenResponse>
	{
		let form = [
			("grant_type", "authorization_code"),
			("client_id", client_id),
			("client_secret", client_secret),
			("redirect_uri", redirect_uri),
			("code", code),
		];
		self.request_token(&form).await
	}

	// Use a refresh token to obtain a fresh access token.
	pub async fn refresh_access_token(
		&self,
		client_id: &str,
		client_secret: &str,
		refresh_token: &str,
	) -> AppResult<TokenResponse>
	{
		let form = [
			("grant_type", "refresh_token"),
			("client_id", client_id),
			("client_secret", client_secret),
			("refresh_token", refresh_token),
		];
		self.request_token(&form).await
	}

	// Shared token-endpoint POST used by both code-exchange and refresh.
	async fn request_token(&self, form: &[(&str, &str)]) -> AppResult<TokenResponse>
	{
		let token_url = format!("{}/oauth/token", self.base_url);
		self.logger.network(&format!("POST {token_url} (oauth token request)"));

		let response = self
			.http_client
			.post(&token_url)
			.header("Accept", "application/json")
			.form(form)
			.send()
			.await?;

		let status = response.status();
		let body_text = response.text().await?;
		self.logger
			.network(&format!("POST {token_url} -> {}", status.as_u16()));

		if !status.is_success()
		{
			return Err(AppError::Network(format!(
				"HMRC token request failed ({}): {body_text}",
				status.as_u16()
			)));
		}

		let token: TokenResponse = serde_json::from_str(&body_text)?;
		Ok(token)
	}

	// Unauthenticated connectivity check against the public "Hello World" API.
	// Useful to confirm the base URL, TLS and network path before authorising.
	pub async fn hello_world(&self, device_id: &str) -> AppResult<HmrcApiResult>
	{
		self.send(
			reqwest::Method::GET,
			"/hello/world",
			HELLO_WORLD_ACCEPT,
			None,
			None,
			device_id,
		)
		.await
	}

	// List all businesses on the taxpayer's HMRC record. Used to let the user pick
	// their Business ID instead of typing it.
	pub async fn list_businesses(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!("/individuals/business/details/{national_insurance_number}/list");
		self.send(
			reqwest::Method::GET,
			&path,
			BUSINESS_DETAILS_ACCEPT,
			Some(access_token),
			None,
			device_id,
		)
		.await
	}

	// Create or amend the cumulative (year-to-date) self-employment summary for a
	// business and tax year. From tax year 2025-26 onwards HMRC replaced the old
	// per-quarter "period summary" (POST .../period, limited to 2024-25 and
	// earlier) with this cumulative model: each call sends the running totals from
	// the start of the tax year (6 April) up to the period end date. The body is
	// assembled by the command layer from the user's mapped totals.
	pub async fn submit_cumulative_period(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		business_id: &str,
		tax_year: &str,
		period_body: serde_json::Value,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!(
			"/individuals/business/self-employment/{national_insurance_number}/{business_id}/cumulative/{tax_year}"
		);
		self.send(
			reqwest::Method::PUT,
			&path,
			SELF_EMPLOYMENT_BUSINESS_ACCEPT,
			Some(access_token),
			Some(period_body),
			device_id,
		)
		.await
	}

	// ---- Read-only GET endpoints backing the "HMRC Get" screen ----
	//
	// Each returns the raw HmrcApiResult (status + JSON body) without treating a
	// non-2xx as an error, so the UI can show exactly what HMRC reports per card
	// (e.g. a 404 when the application is not subscribed to that API).

	// Business Details: the accounting period, type, commencement date, etc. that
	// HMRC holds for one business.
	pub async fn retrieve_business_details(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		business_id: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!("/individuals/business/details/{national_insurance_number}/{business_id}");
		self.send(reqwest::Method::GET, &path, BUSINESS_DETAILS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Obligations: the quarterly income-and-expenditure update obligations (period
	// boundaries, due dates and open/fulfilled status). Scoped to one business when
	// `business_id`/`type_of_business` are supplied.
	pub async fn get_obligations_quarterly(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		type_of_business: Option<&str>,
		business_id: Option<&str>,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let mut query: Vec<(&str, &str)> = Vec::new();
		if let Some(type_of_business) = type_of_business
		{
			query.push(("typeOfBusiness", type_of_business));
		}
		if let Some(business_id) = business_id
		{
			query.push(("businessId", business_id));
		}
		let path = with_query(
			&format!("/obligations/details/{national_insurance_number}/income-and-expenditure"),
			&query,
		);
		self.send(reqwest::Method::GET, &path, OBLIGATIONS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Obligations: the final-declaration (crystallisation) obligations for a year.
	pub async fn get_obligations_final_declaration(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		tax_year: Option<&str>,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let mut query: Vec<(&str, &str)> = Vec::new();
		if let Some(tax_year) = tax_year
		{
			query.push(("taxYear", tax_year));
		}
		let path = with_query(
			&format!("/obligations/details/{national_insurance_number}/crystallisation"),
			&query,
		);
		self.send(reqwest::Method::GET, &path, OBLIGATIONS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Self-employment: the cumulative (year-to-date) figures HMRC currently holds
	// for a business and tax year — the read-back of what the app PUTs.
	pub async fn get_cumulative(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		business_id: &str,
		tax_year: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!(
			"/individuals/business/self-employment/{national_insurance_number}/{business_id}/cumulative/{tax_year}"
		);
		self.send(reqwest::Method::GET, &path, SELF_EMPLOYMENT_BUSINESS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Self-employment: the annual submission (adjustments, allowances) for a year.
	pub async fn get_annual(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		business_id: &str,
		tax_year: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!(
			"/individuals/business/self-employment/{national_insurance_number}/{business_id}/annual/{tax_year}"
		);
		self.send(reqwest::Method::GET, &path, SELF_EMPLOYMENT_BUSINESS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Self-employment: the list of (legacy, <=2024-25) period summaries for a year.
	pub async fn list_period_summaries(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		business_id: &str,
		tax_year: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!(
			"/individuals/business/self-employment/{national_insurance_number}/{business_id}/period/{tax_year}"
		);
		self.send(reqwest::Method::GET, &path, SELF_EMPLOYMENT_BUSINESS_ACCEPT, Some(access_token), None, device_id)
			.await
	}

	// Generic authenticated GET for additional MTD APIs (BISS, Individual
	// Calculations, Self Assessment Accounts). The caller supplies the full path
	// (including any query string) and the API's Accept version.
	pub async fn get_raw(
		&self,
		access_token: &str,
		path: &str,
		accept_version: &str,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		self.send(reqwest::Method::GET, path, accept_version, Some(access_token), None, device_id)
			.await
	}

	// ---- Sandbox-only test-support (MTD SA Test Support API) ----

	// Create a test business for a test user in the stateful sandbox. Returns the
	// minted businessId in the body. Sandbox only; data auto-purges after 7 days.
	pub async fn create_test_business(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		body: serde_json::Value,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!("/individuals/self-assessment-test-support/business/{national_insurance_number}");
		self.send(reqwest::Method::POST, &path, TEST_SUPPORT_ACCEPT, Some(access_token), Some(body), device_id)
			.await
	}

	// Create/amend the test ITSA status for a test user and tax year, so that
	// obligations are generated for stateful testing.
	pub async fn create_test_itsa_status(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		tax_year: &str,
		body: serde_json::Value,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let path = format!(
			"/individuals/self-assessment-test-support/itsa-status/{national_insurance_number}/{tax_year}"
		);
		self.send(reqwest::Method::POST, &path, TEST_SUPPORT_ACCEPT, Some(access_token), Some(body), device_id)
			.await
	}

	// Core request helper: attaches Accept, optional bearer auth, optional JSON
	// body and the fraud-prevention headers, then logs and returns the result.
	async fn send(
		&self,
		method: reqwest::Method,
		path: &str,
		accept_version: &str,
		access_token: Option<&str>,
		json_body: Option<serde_json::Value>,
		device_id: &str,
	) -> AppResult<HmrcApiResult>
	{
		let url = format!("{}{path}", self.base_url);
		self.logger
			.network(&format!("{method} {url} (auth={})", access_token.is_some()));

		let mut builder = self
			.http_client
			.request(method.clone(), &url)
			.header("Accept", accept_version);

		// Attach the bearer token for user-restricted endpoints only.
		if let Some(token) = access_token
		{
			builder = builder.bearer_auth(token);
		}

		// Add the fraud-prevention headers required on every MTD call.
		for (name, value) in fraud_headers::build(device_id)
		{
			builder = builder.header(name, value);
		}

		// Select a stubbed HMRC response / stateful behaviour via Gov-Test-Scenario
		// when the client was built with one (sandbox + mock identity).
		if let Some(scenario) = self.gov_test_scenario.as_deref().filter(|value| !value.is_empty())
		{
			builder = builder.header("Gov-Test-Scenario", scenario);
			self.logger.network(&format!("Gov-Test-Scenario: {scenario}"));
		}

		// Attach a JSON body when present.
		if let Some(body) = json_body
		{
			builder = builder.json(&body);
		}

		let response = builder.send().await?;
		let status = response.status().as_u16();
		let body_text = response.text().await?;
		self.logger.network(&format!("{method} {url} -> {status}"));

		// On any non-2xx, log a snippet of HMRC's response body so the exact error
		// code/message is visible for diagnosis (error bodies carry no secrets).
		if !(200..300).contains(&status)
		{
			let snippet: String = body_text.chars().take(600).collect();
			self.logger.network(&format!("{method} {url} body: {snippet}"));
		}

		// Parse the body as JSON, falling back to a string wrapper so the caller
		// always receives structured data even on a plain-text error page.
		let body = serde_json::from_str(&body_text)
			.unwrap_or_else(|_| serde_json::json!({ "raw": body_text }));

		Ok(HmrcApiResult { status, body })
	}
}
