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
use chrono::Datelike;
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

	let (environment, access_token, raw_nino, device_id, using_mock_identity, gov_test_scenario) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.access_token.clone(),
			settings.hmrc.national_insurance_number.clone(),
			settings.device_id.clone(),
			settings.hmrc.using_mock_identity,
			settings.hmrc.gov_test_scenario.clone(),
		)
	};

	// In the sandbox with a mock identity, the configured scenario (e.g. STATEFUL)
	// rides on the call; otherwise no scenario header.
	let scenario = effective_scenario(&environment, using_mock_identity, &gov_test_scenario);

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

	let client = HmrcClient::new(&environment, state.logger.clone(), scenario);
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
	let client = HmrcClient::new(&environment, state.logger.clone(), None);
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

	let client = HmrcClient::new(&environment, state.logger.clone(), None);
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

	let client = HmrcClient::new(&environment, state.logger.clone(), None);
	let token = client
		.refresh_access_token(&client_id, &client_secret, &refresh_token)
		.await?;

	store_tokens(&state, token)?;
	current_status(&state)
}

// Submit a cumulative (year-to-date) self-employment summary to HMRC from the
// user's mapped income/expense totals. The caller supplies only the date being
// reported up to; the period always starts at the tax-year start (6 April) and
// the figures are aggregated over that whole window, as the cumulative model
// requires. The attempt is always recorded in history, success or failure.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_submit_period(
	state: State<'_, AppState>,
	period_end: String,
) -> AppResult<HmrcSubmission>
{
	// The tax year (and its 6 April start date) are derived from the end date.
	let (period_start, tax_year) = uk_tax_year(&period_end)?;

	state
		.logger
		.action(&format!("HMRC submit cumulative {tax_year} ({period_start}..{period_end})"));

	// Snapshot the HMRC config we need for the call.
	let (environment, access_token, nino, business_id, device_id, using_mock_identity, gov_test_scenario) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.access_token.clone(),
			settings.hmrc.national_insurance_number.clone(),
			settings.hmrc.business_id.clone(),
			settings.device_id.clone(),
			settings.hmrc.using_mock_identity,
			settings.hmrc.gov_test_scenario.clone(),
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

	// Aggregate mapped totals from the tax-year start to the end date (cumulative)
	// inside a short-lived db lock.
	let (totals, unmapped) = {
		let database = state.lock_database()?;
		let totals = database.period_totals_by_hmrc_code(&period_start, &period_end)?;
		let unmapped = database.unmapped_event_count(&period_start, &period_end)?;
		(totals, unmapped)
	};

	// Unmapped events cannot be attributed to an HMRC box, so they are excluded;
	// note it in the action log so the omission is traceable.
	if unmapped > 0
	{
		state.logger.action(&format!(
			"warning: {unmapped} event(s) excluded from the {tax_year} submission for lack of an HMRC mapping"
		));
	}

	let period_body = build_cumulative_body(&period_start, &period_end, &totals);
	let request_json = serde_json::to_string_pretty(&period_body).unwrap_or_default();

	// Perform the network call with no locks held. In stateful sandbox mode the
	// configured scenario (e.g. STATEFUL) rides on the submission too.
	let scenario = effective_scenario(&environment, using_mock_identity, &gov_test_scenario);
	let client = HmrcClient::new(&environment, state.logger.clone(), scenario);
	let api_result = client
		.submit_cumulative_period(
			&access_token,
			&nino,
			&business_id,
			&tax_year,
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
		.or_else(|| api_result.body.get("transactionReference"))
		.and_then(|value| value.as_str())
		.unwrap_or("")
		.to_string();
	let response_json = serde_json::to_string_pretty(&api_result.body).unwrap_or_default();

	// Record the attempt in history (storing the actual submitted window).
	let mut database = state.lock_database()?;
	database.record_submission(
		&period_start,
		&period_end,
		&status,
		&reference,
		&request_json,
		&response_json,
	)
}

// Derive the UK tax year for a date. The tax year starts on 6 April: dates on or
// after 6 April belong to the year beginning that calendar year, earlier dates to
// the previous one. Returns the 6 April start date ("YYYY-04-06") and the
// HMRC tax-year label ("YYYY-YY").
fn uk_tax_year(end_date: &str) -> AppResult<(String, String)>
{
	let date = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|_| {
		AppError::Validation(format!(
			"invalid period end date '{end_date}' (expected YYYY-MM-DD)"
		))
	})?;

	let year = date.year();
	let start_year = if date.month() > 4 || (date.month() == 4 && date.day() >= 6)
	{
		year
	}
	else
	{
		year - 1
	};

	let start_date = format!("{start_year}-04-06");
	let tax_year = format!("{start_year}-{:02}", (start_year + 1) % 100);
	Ok((start_date, tax_year))
}

// Assemble the HMRC cumulative-summary request body from grouped pence totals.
// Empty income/expense objects are omitted (HMRC rejects empty sub-objects), and
// no diagnostic fields are sent — only the keys HMRC defines.
fn build_cumulative_body(
	period_start: &str,
	period_end: &str,
	totals: &[(String, String, i64)],
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

	let mut body = serde_json::Map::new();
	body.insert(
		"periodDates".to_string(),
		serde_json::json!({
			"periodStartDate": period_start,
			"periodEndDate": period_end
		}),
	);
	if !income.is_empty()
	{
		body.insert("periodIncome".to_string(), serde_json::Value::Object(income));
	}
	if !expenses.is_empty()
	{
		body.insert("periodExpenses".to_string(), serde_json::Value::Object(expenses));
	}

	serde_json::Value::Object(body)
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

// ---- Read-only "HMRC Get" commands ----
//
// These retrieve HMRC's authoritative state. Each returns the raw HmrcApiResult
// (HTTP status + JSON body) and does NOT turn a non-2xx into an error, so the UI
// can show exactly what HMRC reports per card — including a 404 when the
// application is not subscribed to the relevant API.

// The effective Gov-Test-Scenario to send on API calls: only in the sandbox, only
// when the user is testing a mock identity, and only when a scenario is set.
fn effective_scenario(environment: &str, using_mock_identity: bool, gov_test_scenario: &str) -> Option<String>
{
	let is_sandbox = environment != "production";
	if is_sandbox && using_mock_identity && !gov_test_scenario.trim().is_empty()
	{
		Some(gov_test_scenario.trim().to_string())
	}
	else
	{
		None
	}
}

// The HMRC config common to every read call, snapshotted out of the settings lock.
struct HmrcReadContext
{
	environment: String,
	access_token: String,
	nino: String,
	business_id: String,
	device_id: String,
	scenario: Option<String>,
}

// Snapshot + validate the config a read call needs. `require_business` enforces a
// configured Business ID for the per-business endpoints.
fn hmrc_read_context(state: &State<'_, AppState>, require_business: bool) -> AppResult<HmrcReadContext>
{
	let (environment, access_token, raw_nino, business_id, device_id, using_mock_identity, gov_test_scenario) = {
		let settings = state.lock_settings()?;
		(
			settings.hmrc.environment.clone(),
			settings.hmrc.access_token.clone(),
			settings.hmrc.national_insurance_number.clone(),
			settings.hmrc.business_id.clone(),
			settings.device_id.clone(),
			settings.hmrc.using_mock_identity,
			settings.hmrc.gov_test_scenario.clone(),
		)
	};

	if access_token.is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"Authorise the app with HMRC first (HMRC Connection screen).".to_string(),
		));
	}
	let nino = normalize_nino(&raw_nino);
	validate_nino(&nino)?;
	if require_business && business_id.trim().is_empty()
	{
		return Err(AppError::HmrcNotConfigured(
			"Set your Business ID on the HMRC Connection screen first.".to_string(),
		));
	}

	let scenario = effective_scenario(&environment, using_mock_identity, &gov_test_scenario);
	Ok(HmrcReadContext { environment, access_token, nino, business_id, device_id, scenario })
}

// Retrieve the business details HMRC holds for the configured business.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_business_details(state: State<'_, AppState>) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action("HMRC get business details");
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	client
		.retrieve_business_details(&context.access_token, &context.nino, &context.business_id, &context.device_id)
		.await
}

// Retrieve the quarterly (income & expenditure) obligations for the business.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_obligations_quarterly(state: State<'_, AppState>) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action("HMRC get quarterly obligations");
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	client
		.get_obligations_quarterly(
			&context.access_token,
			&context.nino,
			Some("self-employment"),
			Some(&context.business_id),
			&context.device_id,
		)
		.await
}

// Retrieve the final-declaration (crystallisation) obligations for a tax year
// (pass an empty string to let HMRC default to the last four years).
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_obligations_final_declaration(
	state: State<'_, AppState>,
	tax_year: String,
) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, false)?;
	state.logger.action("HMRC get final-declaration obligations");
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	let tax_year = if tax_year.trim().is_empty() { None } else { Some(tax_year.trim()) };
	client
		.get_obligations_final_declaration(&context.access_token, &context.nino, tax_year, &context.device_id)
		.await
}

// Retrieve the cumulative (year-to-date) self-employment summary HMRC holds.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_cumulative(state: State<'_, AppState>, tax_year: String) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action(&format!("HMRC get cumulative {tax_year}"));
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	client
		.get_cumulative(&context.access_token, &context.nino, &context.business_id, tax_year.trim(), &context.device_id)
		.await
}

// Retrieve the self-employment annual submission for a tax year.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_annual(state: State<'_, AppState>, tax_year: String) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action(&format!("HMRC get annual {tax_year}"));
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	client
		.get_annual(&context.access_token, &context.nino, &context.business_id, tax_year.trim(), &context.device_id)
		.await
}

// Retrieve the list of (legacy) period summaries for a tax year.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_period_summaries(state: State<'_, AppState>, tax_year: String) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action(&format!("HMRC get period summaries {tax_year}"));
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	client
		.list_period_summaries(&context.access_token, &context.nino, &context.business_id, tax_year.trim(), &context.device_id)
		.await
}

// ---- Phase 2: optional APIs (need their own Developer Hub subscription) ----
// These return HMRC's response as-is; a 404 means the application is not
// subscribed to that API yet.

// Business Income Source Summary (BISS): HMRC's computed income/expense summary
// for one self-employment business and tax year.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_biss(state: State<'_, AppState>, tax_year: String) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, true)?;
	state.logger.action(&format!("HMRC get BISS {tax_year}"));
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	let path = format!(
		"/individuals/self-assessment/income-summary/{}/self-employment/{}/{}",
		context.nino,
		tax_year.trim(),
		context.business_id,
	);
	client
		.get_raw(&context.access_token, &path, crate::hmrc::BISS_ACCEPT, &context.device_id)
		.await
}

// Individual Calculations: the list of tax calculations HMRC holds for a year.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_calculations(state: State<'_, AppState>, tax_year: String) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, false)?;
	state.logger.action(&format!("HMRC get calculations {tax_year}"));
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	let path = format!("/individuals/calculations/{}/self-assessment/{}", context.nino, tax_year.trim());
	client
		.get_raw(&context.access_token, &path, crate::hmrc::CALCULATIONS_ACCEPT, &context.device_id)
		.await
}

// Self Assessment Accounts: the customer's open balance and transactions (what is
// owed/paid). Uses onlyOpenItems=true so no date range is required.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_get_sa_account(state: State<'_, AppState>) -> AppResult<HmrcApiResult>
{
	let context = hmrc_read_context(&state, false)?;
	state.logger.action("HMRC get self assessment account");
	let client = HmrcClient::new(&context.environment, state.logger.clone(), context.scenario.clone());
	let path = format!("/accounts/self-assessment/{}/balance-and-transactions?onlyOpenItems=true", context.nino);
	client
		.get_raw(&context.access_token, &path, crate::hmrc::SA_ACCOUNTS_ACCEPT, &context.device_id)
		.await
}

// ---- Sandbox-only: seed stateful test data (MTD SA Test Support API) ----

// The outcome of a test-data setup run, shown to the user.
#[derive(Debug, Serialize)]
pub struct TestDataSetup
{
	pub business_id: String,
	pub business_created: bool,
	pub itsa_status_http: u16,
	// RFC3339 instant this run seeded the data (HMRC keeps it ~7 days).
	pub seeded_at: String,
	pub message: String,
}

// Idempotently provision a stateful test business + ITSA status for the test user,
// so submissions read back and obligations generate. Sandbox only. Re-running
// reuses an existing self-employment business rather than creating a duplicate.
// All calls use the STATEFUL scenario regardless of the configured one, since that
// is the whole point of this helper.
#[tauri::command(rename_all = "snake_case")]
pub async fn hmrc_setup_test_data(state: State<'_, AppState>, tax_year: String) -> AppResult<TestDataSetup>
{
	let context = hmrc_read_context(&state, false)?;
	if context.environment == "production"
	{
		return Err(AppError::Validation(
			"Setting up test data is only available in the sandbox.".to_string(),
		));
	}
	state.logger.action(&format!("HMRC set up test data {tax_year}"));

	// Reads use a STATEFUL client (to see the stateful store). The test-support
	// seeding endpoints reject a Gov-Test-Scenario, so they use a scenario-less
	// client.
	let stateful_client =
		HmrcClient::new(&context.environment, state.logger.clone(), Some("STATEFUL".to_string()));
	let support_client = HmrcClient::new(&context.environment, state.logger.clone(), None);

	// 1. Idempotent: reuse an existing self-employment business if one exists.
	let list = stateful_client
		.list_businesses(&context.access_token, &context.nino, &context.device_id)
		.await?;
	let existing = list
		.body
		.get("listOfBusinesses")
		.and_then(|value| value.as_array())
		.and_then(|businesses| {
			businesses.iter().find(|business| {
				business.get("typeOfBusiness").and_then(|t| t.as_str()) == Some("self-employment")
			})
		})
		.and_then(|business| business.get("businessId").and_then(|id| id.as_str()))
		.map(|id| id.to_string());

	let (business_id, business_created) = if let Some(id) = existing
	{
		(id, false)
	}
	else
	{
		let body = serde_json::json!({
			"typeOfBusiness": "self-employment",
			"tradingName": "MyOpenUKTaxApp Test Trade",
			"commencementDate": "2016-09-24",
			"businessAddressLineOne": "1 Test Road",
			"businessAddressCountryCode": "GB",
			"businessAddressPostcode": "M1 1AG"
		});
		let created = support_client
			.create_test_business(&context.access_token, &context.nino, body, &context.device_id)
			.await?;
		if !(200..300).contains(&created.status)
		{
			return Err(AppError::Network(format!(
				"Create test business failed (HTTP {}): {}",
				created.status, created.body
			)));
		}
		let id = created
			.body
			.get("businessId")
			.and_then(|value| value.as_str())
			.ok_or_else(|| AppError::Network("Create test business returned no businessId.".to_string()))?
			.to_string();
		(id, true)
	};

	// Persist the business id (short-lived lock, no await held).
	{
		let mut settings = state.lock_settings()?;
		settings.hmrc.business_id = business_id.clone();
		settings.save(&state.paths)?;
	}

	// 2. Set the ITSA status so obligations are generated (amend-safe to repeat).
	let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
	let itsa_body = serde_json::json!({
		"itsaStatusDetails": [
			{
				"submittedOn": now,
				"status": "MTD Mandated",
				"statusReason": "Sign up - no return available"
			}
		]
	});
	let itsa = support_client
		.create_test_itsa_status(&context.access_token, &context.nino, tax_year.trim(), itsa_body, &context.device_id)
		.await?;

	// Record when the test data was (re)seeded so the UI can show the 7-day expiry.
	let seeded_at = Utc::now().to_rfc3339();
	{
		let database = state.lock_database()?;
		database.set_meta("test_data_seeded_at", &seeded_at)?;
	}

	let message = format!(
		"{} business {business_id}; ITSA status set for {} (HTTP {}).",
		if business_created { "Created test" } else { "Reusing existing" },
		tax_year.trim(),
		itsa.status,
	);

	Ok(TestDataSetup { business_id, business_created, itsa_status_http: itsa.status, seeded_at, message })
}

// When the sandbox test data was last seeded (RFC3339), or None if never. Drives
// the "set up on / expires around" note on the HMRC Connection screen.
#[tauri::command(rename_all = "snake_case")]
pub fn hmrc_test_data_seeded_at(state: State<'_, AppState>) -> AppResult<Option<String>>
{
	state.lock_database()?.get_meta("test_data_seeded_at")
}
