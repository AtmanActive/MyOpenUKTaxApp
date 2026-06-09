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
}

impl HmrcClient
{
	// Construct a client for the given environment ("sandbox" | "production").
	// The sandbox-only `Gov-Test-Scenario` header is not a client-wide concern: it
	// is passed per call and only the business-lookup uses it (see `list_businesses`).
	pub fn new(environment: &str, logger: Arc<Logger>) -> Self
	{
		let is_sandbox = environment != "production";
		let base_url = if is_sandbox { SANDBOX_BASE_URL } else { PRODUCTION_BASE_URL }.to_string();

		Self {
			http_client: reqwest::Client::new(),
			base_url,
			logger,
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
			None,
		)
		.await
	}

	// List all businesses on the taxpayer's HMRC record. Used by the Settings
	// screen to let the user pick their Business ID instead of typing it. This is
	// the one call that may carry a `Gov-Test-Scenario` header: in the sandbox,
	// when the user is testing against a mock identity, `gov_test_scenario` selects
	// the stubbed business data (pass `None` to address a real sandbox identity).
	pub async fn list_businesses(
		&self,
		access_token: &str,
		national_insurance_number: &str,
		device_id: &str,
		gov_test_scenario: Option<&str>,
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
			gov_test_scenario,
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
			None,
		)
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
		gov_test_scenario: Option<&str>,
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

		// Select a stubbed HMRC response via Gov-Test-Scenario when the caller asked
		// for one (sandbox + mock-identity, scoped to the calls that support it).
		if let Some(scenario) = gov_test_scenario.filter(|value| !value.is_empty())
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
