// Commands backing the Settings screen.

use crate::error::AppResult;
use crate::log_debug;
use crate::settings::Settings;
use crate::state::AppState;
use tauri::State;

// Return the current settings (including HMRC config) for the UI to render.
#[tauri::command(rename_all = "snake_case")]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings>
{
	log_debug!(state.logger, "get_settings()");
	let settings = state.lock_settings()?;
	Ok(settings.clone())
}

// Validate and persist updated settings. Backend-managed fields (the device id
// and the OAuth tokens) are preserved from the current settings so the UI form
// cannot accidentally clear them, then the database retention knobs are
// refreshed to match.
#[tauri::command(rename_all = "snake_case")]
pub fn update_settings(
	state: State<'_, AppState>,
	settings: Settings,
) -> AppResult<Settings>
{
	state.logger.action("update settings");

	let mut incoming = settings;

	// Carry over fields the Settings UI does not own.
	{
		let current = state.lock_settings()?;
		incoming.device_id = current.device_id.clone();
		incoming.hmrc.access_token = current.hmrc.access_token.clone();
		incoming.hmrc.refresh_token = current.hmrc.refresh_token.clone();
		incoming.hmrc.token_expires_at_epoch_seconds =
			current.hmrc.token_expires_at_epoch_seconds;
	}

	// Persist to the exe-adjacent JSON file (this also validates the values).
	incoming.save(&state.paths)?;

	// Reflect the new retention/backup tuning in the live database handle.
	{
		let mut database = state.lock_database()?;
		database.update_retention_settings(
			incoming.backup_min_interval_seconds,
			incoming.backups_pruned_after_days,
		);
	}

	// Replace the in-memory copy last, after everything else succeeded.
	{
		let mut current = state.lock_settings()?;
		*current = incoming.clone();
	}

	Ok(incoming)
}
