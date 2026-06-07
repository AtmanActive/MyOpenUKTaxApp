// Commands for the Settings "About" and "Updates" sections.
//
// `app_info` exposes static metadata baked in at compile time. `check_latest_version`
// asks GitHub for the most recent published release and reports whether it is
// newer than the running build (a lightweight check — it does not download or
// install anything; the UI opens the release page for a manual update).

use crate::error::AppError;
use crate::error::AppResult;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

// GitHub endpoints for this repository's latest release.
const GITHUB_LATEST_RELEASE_API: &str =
	"https://api.github.com/repos/AtmanActive/MyOpenUKTaxApp/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/AtmanActive/MyOpenUKTaxApp/releases/latest";

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo
{
	pub name: String,
	pub version: String,
	pub authors: String,
	pub homepage: String,
	pub license: String,
}

// Static application metadata, sourced from the crate manifest at compile time.
#[tauri::command(rename_all = "snake_case")]
pub fn app_info() -> AppInfo
{
	AppInfo {
		name: "MyOpenUKTaxApp".to_string(),
		version: env!("CARGO_PKG_VERSION").to_string(),
		authors: env!("CARGO_PKG_AUTHORS").to_string(),
		homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
		license: env!("CARGO_PKG_LICENSE").to_string(),
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheck
{
	pub current_version: String,
	pub latest_version: String,
	pub update_available: bool,
	pub release_url: String,
}

// Query GitHub for the latest release and compare it with the running version.
#[tauri::command(rename_all = "snake_case")]
pub async fn check_latest_version(state: State<'_, AppState>) -> AppResult<UpdateCheck>
{
	state.logger.action("check for updates");
	let current_version = env!("CARGO_PKG_VERSION").to_string();

	// GitHub requires a User-Agent header on API requests.
	let client = reqwest::Client::new();
	let response = client
		.get(GITHUB_LATEST_RELEASE_API)
		.header("User-Agent", "MyOpenUKTaxApp")
		.header("Accept", "application/vnd.github+json")
		.send()
		.await?;

	let status = response.status();
	state
		.logger
		.network(&format!("GET {GITHUB_LATEST_RELEASE_API} -> {}", status.as_u16()));

	// No releases published yet is a normal, non-error state.
	if status.as_u16() == 404
	{
		return Ok(UpdateCheck {
			current_version,
			latest_version: String::new(),
			update_available: false,
			release_url: RELEASES_PAGE.to_string(),
		});
	}
	if !status.is_success()
	{
		return Err(AppError::Network(format!(
			"GitHub returned HTTP {}",
			status.as_u16()
		)));
	}

	let body: serde_json::Value = response.json().await?;
	let latest_version = body
		.get("tag_name")
		.and_then(|value| value.as_str())
		.unwrap_or("")
		.trim_start_matches('v')
		.to_string();
	let release_url = body
		.get("html_url")
		.and_then(|value| value.as_str())
		.unwrap_or(RELEASES_PAGE)
		.to_string();

	let update_available = is_newer(&latest_version, &current_version);

	Ok(UpdateCheck {
		current_version,
		latest_version,
		update_available,
		release_url,
	})
}

// Compare two dotted version strings; true when `latest` is strictly newer.
fn is_newer(latest: &str, current: &str) -> bool
{
	if latest.is_empty()
	{
		return false;
	}

	let parse = |value: &str| -> Vec<u32> {
		value.split('.').map(|part| part.parse::<u32>().unwrap_or(0)).collect()
	};
	let latest_parts = parse(latest);
	let current_parts = parse(current);

	for index in 0..latest_parts.len().max(current_parts.len())
	{
		let latest_component = latest_parts.get(index).copied().unwrap_or(0);
		let current_component = current_parts.get(index).copied().unwrap_or(0);
		if latest_component != current_component
		{
			return latest_component > current_component;
		}
	}

	false
}
