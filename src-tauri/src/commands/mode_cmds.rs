// Commands for the Sandbox/Production run-mode toggle.
//
// Switching mode is fully live (no restart): the new mode is persisted to its own
// JSON file, recorded in AppState, and the database's active schema is flipped so
// every subsequent query reads/writes the matching schema. The frontend then
// re-reads its data and re-skins itself for the new mode.

use crate::error::AppResult;
use crate::runmode;
use crate::runmode::RunMode;
use crate::state::AppState;
use tauri::State;

// Return the current run mode.
#[tauri::command(rename_all = "snake_case")]
pub fn get_run_mode(state: State<'_, AppState>) -> AppResult<RunMode>
{
	state.current_run_mode()
}

// Switch the run mode: persist the flag, update AppState, and point the database
// at the matching schema. Returns the new mode.
#[tauri::command(rename_all = "snake_case")]
pub fn set_run_mode(state: State<'_, AppState>, mode: RunMode) -> AppResult<RunMode>
{
	state.logger.action(&format!("set run mode: {}", mode.schema()));

	// Persist the flag first so a crash mid-switch cannot leave it ambiguous.
	runmode::save(&state.paths, mode)?;

	// Point the live database connection at the matching schema.
	state.lock_database()?.set_active_schema(mode.schema());

	// Record it in shared state last, after the side effects succeeded.
	{
		let mut current = state
			.run_mode
			.lock()
			.map_err(|_| crate::error::AppError::Internal("run-mode lock was poisoned".to_string()))?;
		*current = mode;
	}

	Ok(mode)
}
