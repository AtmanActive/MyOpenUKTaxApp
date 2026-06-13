// Commands backing the Settings screen.

use crate::error::AppError;
use crate::error::AppResult;
use crate::log_debug;
use crate::logging;
use crate::settings::Settings;
use crate::state::AppState;
use tauri::AppHandle;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

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

	// Carry over fields the Settings UI does not own, and remember the previous
	// log-retention window so we can re-prune live if it changed.
	let previous_logs_pruned_after_days;
	{
		let current = state.lock_settings()?;
		previous_logs_pruned_after_days = current.logs_pruned_after_days;
		incoming.device_id = current.device_id.clone();
		// OAuth tokens are backend-managed for BOTH modes; never let the UI clear them.
		for block in [
			(&mut incoming.hmrc_sandbox, &current.hmrc_sandbox),
			(&mut incoming.hmrc_production, &current.hmrc_production),
		]
		{
			let (into, from) = block;
			into.access_token = from.access_token.clone();
			into.refresh_token = from.refresh_token.clone();
			into.token_expires_at_epoch_seconds = from.token_expires_at_epoch_seconds;
		}
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

	// Log files are otherwise only pruned at startup; re-prune now when the
	// retention window changed so the setting applies immediately, not next run.
	if incoming.logs_pruned_after_days != previous_logs_pruned_after_days
	{
		logging::prune_old_logs(&state.paths, incoming.logs_pruned_after_days)?;
	}

	// Replace the in-memory copy last, after everything else succeeded.
	{
		let mut current = state.lock_settings()?;
		*current = incoming.clone();
	}

	Ok(incoming)
}

// Restart the application. Used by the Settings screen for options that are only
// read at startup (the embedded MCP server), so the user can apply them without
// hunting for the window's close button. `AppHandle::restart` never returns.
#[tauri::command]
pub fn restart_app(app: tauri::AppHandle)
{
	app.restart();
}

// Open the portable Data directory in the OS file explorer.
#[tauri::command(rename_all = "snake_case")]
pub fn open_data_directory(state: State<'_, AppState>, app: AppHandle) -> AppResult<()>
{
	state.logger.action("open data directory");
	open_directory(&app, state.paths.data_directory())
}

// Open the portable Logs directory in the OS file explorer.
#[tauri::command(rename_all = "snake_case")]
pub fn open_logs_directory(state: State<'_, AppState>, app: AppHandle) -> AppResult<()>
{
	state.logger.action("open logs directory");
	open_directory(&app, state.paths.logs_directory())
}

// Ensure a directory exists, then reveal it in the OS file manager via the
// opener plugin (handled in Rust so no frontend path-scope permission is needed).
fn open_directory(app: &AppHandle, directory: std::path::PathBuf) -> AppResult<()>
{
	std::fs::create_dir_all(&directory)?;
	app.opener()
		.open_path(directory.to_string_lossy().to_string(), None::<&str>)
		.map_err(|error| AppError::Io(error.to_string()))
}
