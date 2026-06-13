// Shared application state managed by Tauri and handed to every command.
//
// The database and logger live behind `Arc` so the embedded MCP server thread
// can share the very same instances as the Tauri command handlers. Settings are
// guarded by a Mutex because the user can change them at runtime.

use crate::db::Database;
use crate::error::AppError;
use crate::error::AppResult;
use crate::logging::Logger;
use crate::paths::AppPaths;
use crate::runmode::RunMode;
use crate::settings::Settings;
use crate::window_state::Geometry;
use crate::window_state::WindowState;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

pub struct AppState
{
	pub paths: AppPaths,
	pub settings: Mutex<Settings>,
	pub logger: Arc<Logger>,
	pub database: Arc<Mutex<Database>>,
	// The active run mode (Sandbox / Production). Switchable at runtime; selects the
	// DB schema and the HMRC credentials/endpoint.
	pub run_mode: Mutex<RunMode>,
	// Persisted window geometry/mode, plus the first normal geometry seen this
	// session (the baseline used to tell whether the user moved/resized).
	pub window_state: Mutex<WindowState>,
	pub window_baseline: Mutex<Option<Geometry>>,
	// Monotonic counter that debounces window-state saves: each move/resize event
	// bumps it, and only the latest scheduled save actually writes.
	pub window_save_generation: AtomicU64,
}

impl AppState
{
	// Lock the database, converting a poisoned lock into a domain error rather
	// than panicking, so one failed command cannot take the whole app down.
	pub fn lock_database(&self) -> AppResult<MutexGuard<'_, Database>>
	{
		self.database
			.lock()
			.map_err(|_| AppError::Internal("database lock was poisoned".to_string()))
	}

	// Lock the settings with the same poisoned-lock handling.
	pub fn lock_settings(&self) -> AppResult<MutexGuard<'_, Settings>>
	{
		self.settings
			.lock()
			.map_err(|_| AppError::Internal("settings lock was poisoned".to_string()))
	}

	// The current run mode (copied out of the lock).
	pub fn current_run_mode(&self) -> AppResult<RunMode>
	{
		self.run_mode
			.lock()
			.map(|mode| *mode)
			.map_err(|_| AppError::Internal("run-mode lock was poisoned".to_string()))
	}
}
