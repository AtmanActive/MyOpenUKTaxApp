// Commands backing the Category Mapping screen (user subcategory -> HMRC category).

use crate::db::models::CategoryMapping;
use crate::db::models::NewCategoryMapping;
use crate::error::AppResult;
use crate::log_debug;
use crate::state::AppState;
use tauri::State;

// List every mapping with both sides resolved for display.
#[tauri::command(rename_all = "snake_case")]
pub fn list_category_mappings(state: State<'_, AppState>) -> AppResult<Vec<CategoryMapping>>
{
	log_debug!(state.logger, "list_category_mappings()");
	let database = state.lock_database()?;
	database.list_mappings()
}

// Create or replace the mapping for a subcategory. Many subcategories may point
// at the same HMRC category, so this is an upsert keyed on the subcategory.
#[tauri::command(rename_all = "snake_case")]
pub fn set_category_mapping(
	state: State<'_, AppState>,
	input: NewCategoryMapping,
) -> AppResult<()>
{
	state.logger.action(&format!(
		"map subcategory {} -> hmrc category {}",
		input.subcategory_id, input.hmrc_category_id
	));
	let mut database = state.lock_database()?;
	database.set_mapping(&input)
}

// Remove a mapping.
#[tauri::command(rename_all = "snake_case")]
pub fn delete_category_mapping(state: State<'_, AppState>, id: i64) -> AppResult<()>
{
	state.logger.action(&format!("delete mapping {id}"));
	let mut database = state.lock_database()?;
	database.delete_mapping(id)
}
