// Commands backing the Subcategory Management screen.

use crate::db::models::Subcategory;
use crate::error::AppResult;
use crate::log_debug;
use crate::state::AppState;
use tauri::State;

// List subcategories, optionally filtered to "income" or "expense".
#[tauri::command(rename_all = "snake_case")]
pub fn list_subcategories(
	state: State<'_, AppState>,
	kind: Option<String>,
) -> AppResult<Vec<Subcategory>>
{
	log_debug!(state.logger, "list_subcategories(kind={kind:?})");
	let database = state.lock_database()?;
	database.list_subcategories(kind.as_deref())
}

// Create a new user subcategory.
#[tauri::command(rename_all = "snake_case")]
pub fn create_subcategory(
	state: State<'_, AppState>,
	kind: String,
	name: String,
	description: String,
) -> AppResult<Subcategory>
{
	state
		.logger
		.action(&format!("create subcategory {kind}/{name}"));
	let mut database = state.lock_database()?;
	database.create_subcategory(&kind, &name, &description)
}

// Rename / re-describe an existing subcategory (its kind is immutable).
#[tauri::command(rename_all = "snake_case")]
pub fn update_subcategory(
	state: State<'_, AppState>,
	id: i64,
	name: String,
	description: String,
) -> AppResult<Subcategory>
{
	state.logger.action(&format!("update subcategory {id} -> {name}"));
	let mut database = state.lock_database()?;
	database.update_subcategory(id, &name, &description)
}

// Delete a subcategory; rejected by the data layer if it is already in use.
#[tauri::command(rename_all = "snake_case")]
pub fn delete_subcategory(state: State<'_, AppState>, id: i64) -> AppResult<()>
{
	state.logger.action(&format!("delete subcategory {id}"));
	let mut database = state.lock_database()?;
	database.delete_subcategory(id)
}
