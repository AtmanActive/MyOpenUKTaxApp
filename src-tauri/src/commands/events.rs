// Commands backing the Add Event form and the recorded-events tables.

use crate::db::models::EventFilter;
use crate::db::models::LedgerEvent;
use crate::db::models::NewLedgerEvent;
use crate::error::AppResult;
use crate::log_debug;
use crate::state::AppState;
use tauri::State;

// List events of one kind ("income" | "expense") with an optional date/text filter.
#[tauri::command(rename_all = "snake_case")]
pub fn list_events(
	state: State<'_, AppState>,
	kind: String,
	filter: Option<EventFilter>,
) -> AppResult<Vec<LedgerEvent>>
{
	let filter = filter.unwrap_or_default();
	log_debug!(state.logger, "list_events(kind={kind}, filter={filter:?})");
	let database = state.lock_database()?;
	database.list_events(&kind, &filter)
}

// Fetch one event, used by the Add Event screen's read-only/clone mode.
#[tauri::command(rename_all = "snake_case")]
pub fn get_event(state: State<'_, AppState>, id: i64) -> AppResult<LedgerEvent>
{
	log_debug!(state.logger, "get_event(id={id})");
	let database = state.lock_database()?;
	database.get_event(id)
}

// Create a ledger event from the Add Event form payload.
#[tauri::command(rename_all = "snake_case")]
pub fn create_event(
	state: State<'_, AppState>,
	input: NewLedgerEvent,
) -> AppResult<LedgerEvent>
{
	state.logger.action(&format!(
		"create {} event {} on {}",
		input.kind, input.amount_pence, input.event_date
	));
	let mut database = state.lock_database()?;
	database.create_event(&input)
}

// Delete a ledger event.
#[tauri::command(rename_all = "snake_case")]
pub fn delete_event(state: State<'_, AppState>, id: i64) -> AppResult<()>
{
	state.logger.action(&format!("delete event {id}"));
	let mut database = state.lock_database()?;
	database.delete_event(id)
}
