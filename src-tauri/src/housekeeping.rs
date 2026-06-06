// Shared age-based file pruning.
//
// Both the logging subsystem and the database-backup subsystem need to delete
// files older than a configurable number of days. The logic lives here once so
// the two callers cannot drift apart.

use crate::error::AppResult;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

// Delete every regular file in `directory` whose last-modified time is older
// than `max_age_days`. A value of 0 disables pruning (keep everything).
pub fn prune_directory_by_age(directory: &Path, max_age_days: u32) -> AppResult<()>
{
	// Zero is treated as "retain forever" so the user can opt out of pruning.
	if max_age_days == 0
	{
		return Ok(());
	}

	let max_age = Duration::from_secs(u64::from(max_age_days) * 24 * 60 * 60);
	let now = SystemTime::now();

	// The directory may not exist yet on a first run; that simply means there is
	// nothing to prune.
	let entries = match std::fs::read_dir(directory)
	{
		Ok(entries) => entries,
		Err(_) => return Ok(()),
	};

	for entry in entries
	{
		let entry = entry?;
		let path = entry.path();

		// Only ever consider plain files; never recurse or touch subdirectories.
		if !path.is_file()
		{
			continue;
		}

		let modified = entry.metadata()?.modified()?;

		// duration_since errors if the file is somehow newer than "now"; in that
		// case it is obviously not old enough to prune, so skip it.
		if let Ok(age) = now.duration_since(modified)
		{
			if age > max_age
			{
				// Best-effort removal: ignore individual failures so one locked
				// file cannot abort pruning of the rest.
				let _ = std::fs::remove_file(&path);
			}
		}
	}

	Ok(())
}
