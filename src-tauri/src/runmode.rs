// Application run mode: Sandbox vs Production.
//
// Persisted in its own small JSON file (separate from the main settings) so the
// mode is an independent, easily-inspected flag. The mode drives three things at
// runtime, all switchable without a restart: the active SQLite schema (sandbox /
// production attached databases), which HMRC credentials/endpoint are used, and
// the mode-scoped UI classes.

use crate::error::AppResult;
use crate::paths::AppPaths;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode
{
	Sandbox,
	Production,
}

impl Default for RunMode
{
	fn default() -> Self
	{
		RunMode::Sandbox
	}
}

impl RunMode
{
	// The SQLite attached-schema name backing this mode.
	pub fn schema(self) -> &'static str
	{
		match self
		{
			RunMode::Sandbox => "sandbox",
			RunMode::Production => "production",
		}
	}

	// The HMRC environment string this mode maps to.
	pub fn hmrc_environment(self) -> &'static str
	{
		match self
		{
			RunMode::Sandbox => "sandbox",
			RunMode::Production => "production",
		}
	}
}

// The on-disk shape of the run-mode file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunModeFile
{
	#[serde(default)]
	mode: RunMode,
}

// Load the saved run mode, defaulting to Sandbox if the file is missing or
// unreadable (a corrupt flag must never stop the app starting — and Sandbox is
// the safe default).
pub fn load(paths: &AppPaths) -> RunMode
{
	match std::fs::read_to_string(paths.run_mode_file())
	{
		Ok(raw) => serde_json::from_str::<RunModeFile>(&raw)
			.map(|file| file.mode)
			.unwrap_or_default(),
		Err(_) => RunMode::Sandbox,
	}
}

// Persist the run mode atomically (temp file + rename).
pub fn save(paths: &AppPaths, mode: RunMode) -> AppResult<()>
{
	let file = paths.run_mode_file();
	let temporary_file = file.with_extension("json.tmp");
	let serialized = serde_json::to_string_pretty(&RunModeFile { mode })?;
	std::fs::write(&temporary_file, serialized)?;
	std::fs::rename(&temporary_file, &file)?;
	Ok(())
}
