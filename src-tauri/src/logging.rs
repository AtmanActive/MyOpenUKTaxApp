// Three-channel file logging.
//
// The spec requires three independent log streams written to Logs/Action,
// Logs/Debug and Logs/Network:
//   * Action  - what the user clicked.
//   * Debug   - detailed diagnostics including the source file and line.
//   * Network - HTTP(S) requests and responses for the HMRC client.
//
// Each channel's file is created LAZILY, on its first write, and named with the
// session-start timestamp down to the millisecond, e.g.
// `2026-06-06_14-03-21-512_MyOpenUKTaxApp_Action.log`. Lazy creation means a
// channel that is never written this session (often Network) leaves no file
// behind, so there are no zero-byte logs.
//
// Logging is best-effort: a failure to write a log line must never crash the
// app, so the write methods swallow errors after reporting them to stderr.

use crate::error::AppResult;
use crate::paths::AppPaths;
use chrono::Local;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

// Convenience macro that records a Debug-channel message together with the
// source location of the call site, satisfying the "line numbers and file names
// of code responsible" requirement.
#[macro_export]
macro_rules! log_debug
{
	($logger:expr, $($arg:tt)*) =>
	{
		$logger.debug_at(&format!("{}:{}", file!(), line!()), &format!($($arg)*))
	};
}

// One log channel. The file handle is created on first write so unused channels
// never produce an (empty) file.
struct LogChannel
{
	path: PathBuf,
	// None until the first successful write opens the file.
	file: Mutex<Option<File>>,
}

impl LogChannel
{
	fn new(path: PathBuf) -> Self
	{
		Self { path, file: Mutex::new(None) }
	}

	// Append one timestamped line, opening the file on first use. The open and
	// the first write happen under the same lock, so a created file is never
	// left empty.
	fn write(&self, location: Option<&str>, message: &str)
	{
		let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
		let line = match location
		{
			Some(location) => format!("{timestamp} | {location} | {message}\n"),
			None => format!("{timestamp} | {message}\n"),
		};

		let mut guard = match self.file.lock()
		{
			Ok(guard) => guard,
			Err(error) =>
			{
				eprintln!("log lock poisoned: {error}");
				return;
			}
		};

		// Lazily open (create + append) on first write.
		if guard.is_none()
		{
			match OpenOptions::new().create(true).append(true).open(&self.path)
			{
				Ok(file) => *guard = Some(file),
				Err(error) =>
				{
					eprintln!("log open failed: {error}");
					return;
				}
			}
		}

		if let Some(file) = guard.as_mut()
		{
			if let Err(error) = file.write_all(line.as_bytes())
			{
				eprintln!("log write failed: {error}");
			}
		}
	}
}

pub struct Logger
{
	action: LogChannel,
	debug: LogChannel,
	network: LogChannel,
}

impl Logger
{
	// Prepare the three channels for this session. No files are created here;
	// each is created the first time its channel is written to.
	pub fn new(paths: &AppPaths) -> AppResult<Self>
	{
		// A single session stamp keeps the three channel files visually grouped.
		let session_stamp = Local::now().format("%Y-%m-%d_%H-%M-%S-%3f").to_string();
		let channel_path = |directory: PathBuf, channel: &str| -> PathBuf {
			directory.join(format!("{session_stamp}_MyOpenUKTaxApp_{channel}.log"))
		};

		Ok(Self {
			action: LogChannel::new(channel_path(paths.action_logs_directory(), "Action")),
			debug: LogChannel::new(channel_path(paths.debug_logs_directory(), "Debug")),
			network: LogChannel::new(channel_path(paths.network_logs_directory(), "Network")),
		})
	}

	// Record a user action (a click / navigation / form submission).
	pub fn action(&self, message: &str)
	{
		self.action.write(None, message);
	}

	// Record a diagnostic message with the source location of the call site.
	pub fn debug_at(&self, location: &str, message: &str)
	{
		self.debug.write(Some(location), message);
	}

	// Record a network request/response line for the HMRC client.
	pub fn network(&self, message: &str)
	{
		self.network.write(None, message);
	}
}

// Delete log files older than `max_age_days` across all three channels. Called
// once at startup; failures are returned so the caller can log them.
pub fn prune_old_logs(paths: &AppPaths, max_age_days: u32) -> AppResult<()>
{
	for directory in [
		paths.action_logs_directory(),
		paths.debug_logs_directory(),
		paths.network_logs_directory(),
	]
	{
		crate::housekeeping::prune_directory_by_age(&directory, max_age_days)?;
	}

	Ok(())
}

// Remove any zero-byte `.log` files across the three channels. Cleans up empties
// left by older app versions (and any that ever slip through); safe to run at
// startup because the current session's files are created lazily on first write.
pub fn remove_empty_logs(paths: &AppPaths) -> AppResult<()>
{
	for directory in [
		paths.action_logs_directory(),
		paths.debug_logs_directory(),
		paths.network_logs_directory(),
	]
	{
		let entries = match std::fs::read_dir(&directory)
		{
			Ok(entries) => entries,
			Err(_) => continue,
		};

		for entry in entries.flatten()
		{
			let path = entry.path();
			let is_empty_file = path.is_file()
				&& entry.metadata().map(|metadata| metadata.len() == 0).unwrap_or(false);
			if is_empty_file
			{
				// Best-effort: ignore a file that cannot be removed.
				let _ = std::fs::remove_file(&path);
			}
		}
	}

	Ok(())
}
