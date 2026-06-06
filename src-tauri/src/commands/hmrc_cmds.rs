// Commands backing the HMRC Post History screen and the HMRC connection flow.
//
// Async commands here must never hold a settings/database lock across an
// `.await`, so each one snapshots the values it needs (cloning out of the lock),
// drops the guard, performs the network call, then re-locks to persist results.

use crate::db::models::HmrcCategory;
use crate::db::models::HmrcSubmission;
use crate::error::AppError;
use crate::error::AppResult;
use crate::hmrc::HmrcApiResult;
use crate::hmrc::HmrcClient;
use crate::hmrc::TokenResponse;
use crate::log_debug;
use crate::state::AppState;
use crate::util;
use chrono::Utc;
use serde::Serialize;
use tauri::State;

// A compact view of the HMRC connection state for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct HmrcStatus
{
	// Client id is present, so the OAuth flow can be started.
	pub configured: bool,
	// NINO and business id are present, so submissions can be addressed.
	pub business_configured: bool,
	// An access token has been obtained.
	pub has_token: bool,
	pub environment: String,
	pub token_expires_at_epoch_seconds: i64,
}

// List the locally cached HMRC categories (read-only to the user).
#[tauri::command(rename_all = "snake_case")]
pub fn list_hmrc_categories(
	state: State<'_, AppState>,
	kind: Option<String>,
) -> AppResult<Vec<HmrcCategory>>
{
	log_debug!(state.logger, "list_hmrc_categories(kind={kind:?})");
	let database = state.lock_database()?;
	database.list_hmrc_categories(kind.as_deref())
}

// List the quarterly submission history, newest first.
#[tauri::command(rename_all = "snake_case")]
pub fn list_hmrc_submissions(state: State<'_, AppState>) -> AppResult<Vec<HmrcSubmission>>
{
	log_debug!(state.logger, "list_hmrc_submissions()");
	let database = state.lock_database()?;
	database.list_submissions()
}

// Report the current HMRC connection state.
#[tauri::command(rename_all = "snake_case")]
pub fn hmrc_status(state: State<'_, AppState>) -> AppResult<HmrcStatus>
{
	current_status(&state)
}

// Build the URL the user must open to authorise the app. A random `state` value
// is generated for CSRF protection; verifying it on the callback is a follow-up
// once the local redirect listener is implemented.
#[tauri::command(rename_all = "snake_case")]
pub fn hmrc_authorize_url(state: State<'_, AppState>) -> AppResult<String>
{
	let settings = state.lock_settings()?;
	if settings.hmrc.client_id.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"set the HMRC client id in Settings first".to_string(),
		));
	}

	let client = HmrcClient::new(&settings.hmrc.environment, state.logger.clone());
	let csrf_state = util::random_hex_token(24);
	let url = client.authorize_url(
		&settings.hmrc.client_id,
		&settings.hmrc.redirect_uri,
		&csrf_state,
	);

	state.logger.action("requested HMRC authorize URL");
	Ok(url)
}

// Unauthenticated sandbox/production connectivity check.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_hello_world(state: State<'_, AppState>) -> AppResult<HmrcApiResult>
{
	state.logger.action("HMRC connectivity test");

	// Snapshot the values we need, then drop the guard before awaiting.
	let (environment, device_id) = {
		let settings = state.lock_settings()?;
		(settings.hmrc.environment.clone(), settings.device_id.clone())
	};

	let client = HmrcClient::new(&environment, state.logger.clone());
	client.hello_world(&device_id).await
}

// Exchange an authorisation code (pasted/redirected back from HMRC) for tokens.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_exchange_code(
	state: State<'_, AppState>,
	code: String,
) -> AppResult<HmrcStatus>
{
	state.logger.action("HMRC exchange authorization code");

	let (environment, client_id, client_secret, redirect_uri) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.client_id.clone(),
			settings.hmrc.client_secret.clone(),
			settings.hmrc.redirect_uri.clone(),
		)
	};

	if client_id.is_empty() || client_secret.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"set the HMRC client id and secret in Settings first".to_string(),
		));
	}

	let client = HmrcClient::new(&environment, state.logger.clone());
	let token = client
		.exchange_code(&client_id, &client_secret, &redirect_uri, &code)
		.await?;

	store_tokens(&state, token)?;
	current_status(&state)
}

// Refresh the access token using the stored refresh token.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_refresh_token(state: State<'_, AppState>) -> AppResult<HmrcStatus>
{
	state.logger.action("HMRC refresh token");

	let (environment, client_id, client_secret, refresh_token) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.client_id.clone(),
			settings.hmrc.client_secret.clone(),
			settings.hmrc.refresh_token.clone(),
		)
	};

	if refresh_token.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"no refresh token yet; authorise the app first".to_string(),
		));
	}

	let client = HmrcClient::new(&environment, state.logger.clone());
	let token = client
		.refresh_access_token(&client_id, &client_secret, &refresh_token)
		.await?;

	store_tokens(&state, token)?;
	current_status(&state)
}

// Build and submit a quarterly self-employment period summary to HMRC from the
// user's mapped income/expense totals over the chosen window. The attempt is
// always recorded in the submission history, whether it succeeds or fails.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_submit_period(
	state: State<'_, AppState>,
	period_from: String,
	period_to: String,
) -> AppResult<HmrcSubmission>
{
	state
		.logger
		.action(&format!("HMRC submit period {period_from}..{period_to}"));

	// Snapshot the HMRC config we need for the call.
	let (environment, access_token, nino, business_id, device_id) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.access_token.clone(),
			settings.hmrc.national_insurance_number.clone(),
			settings.hmrc.business_id.clone(),
			settings.device_id.clone(),
		)
	};

	if access_token.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"authorise the app with HMRC before submitting".to_string(),
		));
	}
	if nino.is_empty() || business_id.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"set your National Insurance number and business id in Settings".to_string(),
		));
	}

	// Aggregate mapped totals for the window inside a short-lived db lock.
	let (totals, unmapped) = {
		let database = state.lock_database()?;
		let totals = database.period_totals_by_hmrc_code(&period_from, &period_to)?;
		let unmapped = database.unmapped_event_count(&period_from, &period_to)?;
		(totals, unmapped)
	};

	let period_body = build_period_body(&period_from, &period_to, &totals, unmapped);
	let request_json = serde_json::to_string_pretty(&period_body).unwrap_or_default();

	// Perform the network call with no locks held.
	let client = HmrcClient::new(&environment, state.logger.clone());
	let api_result = client
		.submit_self_employment_period(
			&access_token,
			&nino,
			&business_id,
			period_body,
			&device_id,
		)
		.await?;

	// Derive a friendly status and any reference id returned by HMRC.
	let status = if (200..300).contains(&api_result.status)
	{
		"submitted".to_string()
	}
	else
	{
		format!("failed ({})", api_result.status)
	};
	let reference = api_result
		.body
		.get("periodId")
		.and_then(|value| value.as_str())
		.unwrap_or("")
		.to_string();
	let response_json = serde_json::to_string_pretty(&api_result.body).unwrap_or_default();

	// Record the attempt in history.
	let mut database = state.lock_database()?;
	database.record_submission(
		&period_from,
		&period_to,
		&status,
		&reference,
		&request_json,
		&response_json,
	)
}

// Assemble the HMRC period-summary request body from grouped pence totals.
fn build_period_body(
	period_from: &str,
	period_to: &str,
	totals: &[(String, String, i64)],
	unmapped: i64,
) -> serde_json::Value
{
	let mut income = serde_json::Map::new();
	let mut expenses = serde_json::Map::new();

	// HMRC expects pound amounts with two decimal places, so convert pence.
	for (code, kind, total_pence) in totals
	{
		let pounds = (*total_pence as f64) / 100.0;
		let rounded = (pounds * 100.0).round() / 100.0;
		let value = serde_json::json!(rounded);
		if kind == "income"
		{
			income.insert(code.clone(), value);
		}
		else
		{
			expenses.insert(code.clone(), value);
		}
	}

	serde_json::json!({
		"periodDates": {
			"periodStartDate": period_from,
			"periodEndDate": period_to
		},
		"periodIncome": income,
		"periodExpenses": expenses,
		// Diagnostic only (not sent as a real HMRC field), surfaced so the UI can
		// warn that some events were excluded for lack of a mapping.
		"_unmappedEventCount": unmapped
	})
}

// Persist freshly obtained tokens and compute the absolute expiry instant.
fn store_tokens(state: &State<'_, AppState>, token: TokenResponse) -> AppResult<()>
{
	let mut settings = state.lock_settings()?;
	settings.hmrc.access_token = token.access_token;
	// A refresh response may omit a new refresh token; keep the existing one then.
	if !token.refresh_token.is_empty()
	{
		settings.hmrc.refresh_token = token.refresh_token;
	}
	settings.hmrc.token_expires_at_epoch_seconds = Utc::now().timestamp() + token.expires_in;
	settings.save(&state.paths)?;
	Ok(())
}

// Read the current connection status out of settings.
fn current_status(state: &State<'_, AppState>) -> AppResult<HmrcStatus>
{
	let settings = state.lock_settings()?;
	Ok(HmrcStatus {
		configured: !settings.hmrc.client_id.is_empty(),
		business_configured: !settings.hmrc.national_insurance_number.is_empty()
			&& !settings.hmrc.business_id.is_empty(),
		has_token: !settings.hmrc.access_token.is_empty(),
		environment: settings.hmrc.environment.clone(),
		token_expires_at_epoch_seconds: settings.hmrc.token_expires_at_epoch_seconds,
	})
}
