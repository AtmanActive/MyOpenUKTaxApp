// Commands backing the HMRC Post History screen and the HMRC connection flow.
//
// Async commands here must never hold a settings/database lock across an
// `.await`, so each one snapshots the values it needs (cloning out of the lock),
// drops the guard, performs the network call, then re-locks to persist results.

use crate::db::models::HmrcCategory;
use crate::db::models::HmrcSubmission;
use crate::error::AppError;
use crate::error::AppResult;
use crate::hmrc::redirect_uri_for_port;
use crate::hmrc::HmrcApiResult;
use crate::hmrc::HmrcClient;
use crate::hmrc::TokenResponse;
use crate::log_debug;
use crate::state::AppState;
use crate::util;
use chrono::Utc;
use serde::Serialize;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

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

// One business on the taxpayer's HMRC record.
#[derive(Debug, Clone, Serialize)]
pub struct HmrcBusiness
{
	pub business_id: String,
	pub type_of_business: String,
	pub trading_name: String,
}

// Fetch the list of businesses from HMRC so the user can pick their Business ID.
// This read-only call also doubles as the NINO-to-account authorisation check:
// a 403 here means the signed-in account is not authorised for the entered NINO.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_list_businesses(state: State<'_, AppState>) -> AppResult<Vec<HmrcBusiness>>
{
	state.logger.action("HMRC list businesses");

	let (environment, access_token, raw_nino, device_id) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.access_token.clone(),
			settings.hmrc.national_insurance_number.clone(),
			settings.device_id.clone(),
		)
	};

	if access_token.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"Authorise the app with HMRC before fetching businesses.".to_string(),
		));
	}
	if raw_nino.trim().is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"Enter your National Insurance number in Settings first.".to_string(),
		));
	}

	// Normalise and sanity-check the NINO so obvious typos fail instantly,
	// without a network round-trip.
	let nino = normalize_nino(&raw_nino);
	validate_nino(&nino)?;

	let client = HmrcClient::new(
		&environment,
		state.logger.clone(),
		&state.lock_settings()?.hmrc.gov_test_scenario,
	);
	let result = client.list_businesses(&access_token, &nino, &device_id).await?;

	// Translate the HTTP status into a clear, actionable message.
	match result.status
	{
		200..=299 => {}
		400 => return Err(AppError::Validation(format!(
			"HMRC rejected National Insurance number {nino}. Double-check that it is correct."
		))),
		401 => return Err(AppError::HmrcNotConfigured(
			"Your HMRC sign-in has expired. Please authorise again.".to_string(),
		)),
		403 => return Err(AppError::Validation(format!(
			"The HMRC account you signed in with is not authorised for National Insurance number \
			 {nino}. Sign in as the right person, or use an account with agent authorisation for \
			 that NINO."
		))),
		404 => return Err(AppError::NotFound(
			"HMRC returned no business details for this account/NINO. In the sandbox, set a \
			 Gov-Test-Scenario in Settings (or use a test user that has a business set up)."
				.to_string(),
		)),
		status => return Err(AppError::Network(format!(
			"HMRC returned HTTP {status} when listing businesses: {}",
			result.body
		))),
	}

	// The response carries a `listOfBusinesses` array; map each entry, tolerating
	// missing optional fields like the trading name.
	let mut businesses = Vec::new();
	if let Some(list) = result.body.get("listOfBusinesses").and_then(|value| value.as_array())
	{
		for item in list
		{
			let string_field = |key: &str| -> String {
				item.get(key).and_then(|value| value.as_str()).unwrap_or("").to_string()
			};
			businesses.push(HmrcBusiness {
				business_id: string_field("businessId"),
				type_of_business: string_field("typeOfBusiness"),
				trading_name: string_field("tradingName"),
			});
		}
	}

	Ok(businesses)
}

// Persist the chosen Business ID (used by the post-authorise auto-prefill when a
// single business is found).
#[tauri::command(rename_all = "snake_case")]
pub fn hmrc_set_business_id(state: State<'_, AppState>, business_id: String) -> AppResult<()>
{
	state.logger.action("HMRC set business id");
	let mut settings = state.lock_settings()?;
	settings.hmrc.business_id = business_id;
	settings.save(&state.paths)?;
	Ok(())
}

// Normalise a NINO: drop any whitespace and upper-case it.
fn normalize_nino(raw: &str) -> String
{
	raw.chars()
		.filter(|character| !character.is_whitespace())
		.collect::<String>()
		.to_uppercase()
}

// Light shape check: two letters, six digits, then a suffix letter A–D (e.g.
// AB123456C). Deliberately not the full HMRC rule set — just enough to catch
// typos before a network call.
fn validate_nino(nino: &str) -> AppResult<()>
{
	let bytes = nino.as_bytes();
	let well_formed = nino.len() == 9
		&& bytes[0..2].iter().all(u8::is_ascii_uppercase)
		&& bytes[2..8].iter().all(u8::is_ascii_digit)
		&& matches!(bytes[8], b'A'..=b'D');

	if well_formed
	{
		Ok(())
	}
	else
	{
		Err(AppError::Validation(format!(
			"'{nino}' does not look like a National Insurance number (expected like AB123456C)."
		)))
	}
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

// The candidate loopback redirect URIs the user must register with their HMRC
// application (one per configured fallback port).
#[tauri::command(rename_all = "snake_case")]
pub fn hmrc_redirect_uris(state: State<'_, AppState>) -> AppResult<Vec<String>>
{
	let settings = state.lock_settings()?;
	Ok(settings
		.hmrc
		.oauth_redirect_ports
		.iter()
		.map(|port| redirect_uri_for_port(*port))
		.collect())
}

// Run the whole authorisation-code flow automatically: bind the first free
// configured loopback port, open HMRC sign-in in the browser, capture the
// redirected code (verifying the CSRF state), exchange it for tokens and store
// them. No copy-paste required.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_authorize(app: AppHandle, state: State<'_, AppState>) -> AppResult<HmrcStatus>
{
	state.logger.action("HMRC authorise (automatic)");

	let (environment, client_id, client_secret, ports) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.client_id.clone(),
			settings.hmrc.client_secret.clone(),
			settings.hmrc.oauth_redirect_ports.clone(),
		)
	};

	if client_id.is_empty() || client_secret.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"set the HMRC client id and secret in Settings first".to_string(),
		));
	}

	// Bind the first free configured port so the redirect URI matches a
	// registered one, then derive that exact URI.
	let (servers, port) = bind_loopback_servers(&ports)?;
	let redirect_uri = redirect_uri_for_port(port);
	let csrf_state = util::random_hex_token(24);

	// Open the HMRC sign-in page in the default browser.
	let client = HmrcClient::new(
		&environment,
		state.logger.clone(),
		&state.lock_settings()?.hmrc.gov_test_scenario,
	);
	let authorize_url = client.authorize_url(&client_id, &redirect_uri, &csrf_state);
	app.opener()
		.open_url(authorize_url, None::<&str>)
		.map_err(|error| AppError::Io(error.to_string()))?;

	// Wait on a blocking thread for the browser to be redirected back with a code.
	let expected_state = csrf_state.clone();
	let code = tauri::async_runtime::spawn_blocking(move || {
		wait_for_oauth_code(servers, &expected_state, Duration::from_secs(180))
	})
	.await
	.map_err(|error| AppError::Internal(format!("authorisation listener failed: {error}")))??;

	// The redirect has landed; bring our window to the front so the user sees the
	// result without having to dismiss the browser themselves.
	bring_window_to_front(&app);

	// Exchange the code (using the same redirect URI) and persist the tokens.
	let token = client
		.exchange_code(&client_id, &client_secret, &redirect_uri, &code)
		.await?;
	store_tokens(&state, token)?;
	current_status(&state)
}

// Raise and focus the main window (unminimising/showing it first if needed).
fn bring_window_to_front(app: &AppHandle)
{
	if let Some(window) = app.get_webview_window("main")
	{
		let _ = window.unminimize();
		let _ = window.show();
		let _ = window.set_focus();
	}
}

// Bind loopback listeners for the first free configured port. Both the IPv4
// (127.0.0.1) and IPv6 (::1) loopback are bound where possible, because some
// browsers resolve "localhost" to ::1 and will not fall back to IPv4.
fn bind_loopback_servers(ports: &[u16]) -> AppResult<(Vec<tiny_http::Server>, u16)>
{
	for &port in ports
	{
		let mut servers = Vec::new();
		if let Ok(server) = tiny_http::Server::http(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
		{
			servers.push(server);
		}
		if let Ok(server) = tiny_http::Server::http(SocketAddr::from((Ipv6Addr::LOCALHOST, port)))
		{
			servers.push(server);
		}
		if !servers.is_empty()
		{
			return Ok((servers, port));
		}
	}

	Err(AppError::Internal(
		"no free OAuth redirect port available; close other apps using them or add more ports"
			.to_string(),
	))
}

// Build the small HTML page shown in the browser once the redirect is received.
fn oauth_done_response() -> tiny_http::Response<std::io::Cursor<Vec<u8>>>
{
	let body = "<html><body style=\"font-family:sans-serif;padding:2rem\">\
		<h2>MyOpenUKTaxApp</h2><p>Authorisation received. You can close this tab \
		and return to the app.</p></body></html>";
	let mut response = tiny_http::Response::from_string(body);
	if let Ok(header) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
	{
		response.add_header(header);
	}
	response
}

// Block until the OAuth redirect arrives on any bound loopback listener (or the
// timeout elapses), returning the code once the CSRF state has been verified.
// One thread listens per bound address (IPv4/IPv6); the first to see a valid
// callback wins.
fn wait_for_oauth_code(
	servers: Vec<tiny_http::Server>,
	expected_state: &str,
	timeout: Duration,
) -> AppResult<String>
{
	let (sender, receiver) = mpsc::channel::<AppResult<String>>();
	let stop = Arc::new(AtomicBool::new(false));
	let expected_state = expected_state.to_string();

	let mut handles = Vec::new();
	for server in servers
	{
		let sender = sender.clone();
		let stop = stop.clone();
		let expected_state = expected_state.clone();
		handles.push(std::thread::spawn(move || {
			while !stop.load(Ordering::Relaxed)
			{
				match server.recv_timeout(Duration::from_millis(250))
				{
					Ok(Some(request)) =>
					{
						let (code, returned_state) = parse_callback_query(request.url());
						let _ = request.respond(oauth_done_response());

						// Ignore unrelated requests (e.g. favicon) that carry no code.
						if let Some(code) = code
						{
							let result = if returned_state.as_deref() == Some(expected_state.as_str())
							{
								Ok(code)
							}
							else
							{
								Err(AppError::Validation(
									"OAuth state mismatch — authorisation rejected".to_string(),
								))
							};
							let _ = sender.send(result);
							stop.store(true, Ordering::Relaxed);
							return;
						}
					}
					// Timed out, or a transient error: loop and re-check the stop flag.
					Ok(None) => {}
					Err(_) => {}
				}
			}
		}));
	}

	// Drop our spare sender so the channel can close if every thread exits.
	drop(sender);

	let outcome = match receiver.recv_timeout(timeout)
	{
		Ok(result) => result,
		Err(_) => Err(AppError::Network("HMRC authorisation timed out".to_string())),
	};

	// Tell the listener threads to wind down and wait for them.
	stop.store(true, Ordering::Relaxed);
	for handle in handles
	{
		let _ = handle.join();
	}

	outcome
}

// Extract `code` and `state` from a callback URL like `/oauth/callback?code=…&state=…`.
fn parse_callback_query(url: &str) -> (Option<String>, Option<String>)
{
	let query = match url.split_once('?')
	{
		Some((_, query)) => query,
		None => return (None, None),
	};

	let mut code = None;
	let mut state = None;
	for pair in query.split('&')
	{
		if let Some((key, value)) = pair.split_once('=')
		{
			let decoded = urlencoding::decode(value)
				.map(|value| value.into_owned())
				.unwrap_or_else(|_| value.to_string());
			match key
			{
				"code" => code = Some(decoded),
				"state" => state = Some(decoded),
				_ => {}
			}
		}
	}

	(code, state)
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

	let client = HmrcClient::new(
		&environment,
		state.logger.clone(),
		&state.lock_settings()?.hmrc.gov_test_scenario,
	);
	client.hello_world(&device_id).await
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

	let client = HmrcClient::new(
		&environment,
		state.logger.clone(),
		&state.lock_settings()?.hmrc.gov_test_scenario,
	);
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
	let client = HmrcClient::new(
		&environment,
		state.logger.clone(),
		&state.lock_settings()?.hmrc.gov_test_scenario,
	);
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
