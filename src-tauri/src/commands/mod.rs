// Tauri command handlers grouped by domain.
//
// Each submodule exposes `#[tauri::command]` functions that the React frontend
// invokes by name. All commands use `rename_all = "snake_case"` so the argument
// keys passed from TypeScript match the project's snake_case naming standard.

pub mod app_cmds;
pub mod dashboard;
pub mod events;
pub mod hmrc_cmds;
pub mod mappings;
pub mod settings_cmds;
pub mod subcategories;
