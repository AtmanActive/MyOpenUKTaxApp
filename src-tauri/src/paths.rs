// Portable path discovery.
//
// The app must be fully self-contained: it never assumes any well-known OS
// directory (no AppData, no home dir). Instead it discovers the directory that
// contains the running executable and derives every other path relative to it,
// so the application plus all of its data lives inside a single folder that the
// user can move or copy freely.

use crate::error::AppError;
use crate::error::AppResult;
use std::path::PathBuf;

// The settings file name lives next to the executable, as required by the spec.
const SETTINGS_FILE_NAME: &str = "MyOpenUKTaxApp.settings.json";
const WINDOW_STATE_FILE_NAME: &str = "MyOpenUKTaxApp.window.json";
const RUN_MODE_FILE_NAME: &str = "MyOpenUKTaxApp.runmode.json";
const DATABASE_FILE_NAME: &str = "MyOpenUKTaxApp.db";
// Per-mode database files, attached as the `sandbox` / `production` schemas.
const SANDBOX_DATABASE_FILE_NAME: &str = "MyOpenUKTaxApp.sandbox.db";
const PRODUCTION_DATABASE_FILE_NAME: &str = "MyOpenUKTaxApp.production.db";

#[derive(Debug, Clone)]
pub struct AppPaths
{
	// The directory that contains the executable; the root of everything.
	pub base_directory: PathBuf,
}

impl AppPaths
{
	// Resolve the executable's directory once at startup. Done eagerly so that a
	// failure to locate ourselves is reported immediately rather than later.
	pub fn discover() -> AppResult<Self>
	{
		let executable_path = std::env::current_exe()
			.map_err(|error| AppError::Io(format!("could not locate the executable: {error}")))?;

		let base_directory = executable_path
			.parent()
			.ok_or_else(|| AppError::Io("executable has no parent directory".to_string()))?
			.to_path_buf();

		Ok(Self { base_directory })
	}

	// The exe-adjacent settings JSON file.
	pub fn settings_file(&self) -> PathBuf
	{
		self.base_directory.join(SETTINGS_FILE_NAME)
	}

	// The exe-adjacent window-geometry JSON file (position/size/mode), kept
	// separate from settings so frequent window writes never touch the file that
	// holds HMRC credentials.
	pub fn window_state_file(&self) -> PathBuf
	{
		self.base_directory.join(WINDOW_STATE_FILE_NAME)
	}

	// The exe-adjacent run-mode flag file (Sandbox vs Production).
	pub fn run_mode_file(&self) -> PathBuf
	{
		self.base_directory.join(RUN_MODE_FILE_NAME)
	}

	// The Data/ subdirectory that holds the database and its backups.
	pub fn data_directory(&self) -> PathBuf
	{
		self.base_directory.join("Data")
	}

	// The legacy single SQLite database file at Data/MyOpenUKTaxApp.db. Kept for the
	// one-time migration that moves its contents into the sandbox schema file.
	pub fn database_file(&self) -> PathBuf
	{
		self.data_directory().join(DATABASE_FILE_NAME)
	}

	// The per-mode database files, attached as the `sandbox` / `production` schemas.
	pub fn sandbox_database_file(&self) -> PathBuf
	{
		self.data_directory().join(SANDBOX_DATABASE_FILE_NAME)
	}

	pub fn production_database_file(&self) -> PathBuf
	{
		self.data_directory().join(PRODUCTION_DATABASE_FILE_NAME)
	}

	// The Data/Backups subdirectory holding timestamped database copies.
	pub fn backups_directory(&self) -> PathBuf
	{
		self.data_directory().join("Backups")
	}

	// The Logs/ root directory.
	pub fn logs_directory(&self) -> PathBuf
	{
		self.base_directory.join("Logs")
	}

	// Logs/Action: records what the user clicked.
	pub fn action_logs_directory(&self) -> PathBuf
	{
		self.logs_directory().join("Action")
	}

	// Logs/Debug: detailed diagnostic logging including code locations.
	pub fn debug_logs_directory(&self) -> PathBuf
	{
		self.logs_directory().join("Debug")
	}

	// Logs/Network: HTTP(S) request/response logging for the HMRC client.
	pub fn network_logs_directory(&self) -> PathBuf
	{
		self.logs_directory().join("Network")
	}

	// Create every subdirectory the app relies on. Called once at startup so the
	// rest of the code can assume the directory tree already exists.
	pub fn ensure_directories(&self) -> AppResult<()>
	{
		for directory in [
			self.data_directory(),
			self.backups_directory(),
			self.logs_directory(),
			self.action_logs_directory(),
			self.debug_logs_directory(),
			self.network_logs_directory(),
		]
		{
			std::fs::create_dir_all(&directory)?;
		}

		Ok(())
	}
}
