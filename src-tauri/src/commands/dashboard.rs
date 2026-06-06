// Command backing the Dashboard statistics view.

use crate::db::models::DashboardSummary;
use crate::error::AppResult;
use crate::log_debug;
use crate::state::AppState;
use tauri::State;

// Compute income/expense totals and a per-subcategory breakdown for an optional
// date window (empty bounds mean unbounded on that side).
#[tauri::command(rename_all = "snake_case")]
pub fn get_dashboard_summary(
	state: State<'_, AppState>,
	date_from: Option<String>,
	date_to: Option<String>,
) -> AppResult<DashboardSummary>
{
	log_debug!(
		state.logger,
		"get_dashboard_summary(from={date_from:?}, to={date_to:?})"
	);
	let database = state.lock_database()?;
	database.dashboard_summary(date_from.as_deref(), date_to.as_deref())
}
