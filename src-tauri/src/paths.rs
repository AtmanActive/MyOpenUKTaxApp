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
const DATABASE_FILE_NAME: &str = "MyOpenUKTaxApp.db";

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

	// The Data/ subdirectory that holds the database and its backups.
	pub fn data_directory(&self) -> PathBuf
	{
		self.base_directory.join("Data")
	}

	// The SQLite database file at Data/MyOpenUKTaxApp.db.
	pub fn database_file(&self) -> PathBuf
	{
		self.data_directory().join(DATABASE_FILE_NAME)
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
