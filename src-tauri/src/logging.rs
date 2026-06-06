// Three-channel file logging.
//
// The spec requires three independent log streams written to Logs/Action,
// Logs/Debug and Logs/Network:
//   * Action  - what the user clicked.
//   * Debug   - detailed diagnostics including the source file and line.
//   * Network - HTTP(S) requests and responses for the HMRC client.
//
// One file per channel is opened per app run (session) and appended to as the
// session proceeds. Files are named with the session-start timestamp down to
// the millisecond, e.g. `2026-06-06_14-03-21-512_MyOpenUKTaxApp_Action.log`.
//
// Logging is best-effort: a failure to write a log line must never crash the
// app, so the write methods swallow errors after reporting them to stderr.

use crate::error::AppResult;
use crate::paths::AppPaths;
use chrono::Local;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
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

pub struct Logger
{
	// Each channel guards its own file handle so the three streams never block
	// one another beyond their individual append.
	action_writer: Mutex<File>,
	debug_writer: Mutex<File>,
	network_writer: Mutex<File>,
}

impl Logger
{
	// Open the three per-session log files. The directories are guaranteed to
	// exist by AppPaths::ensure_directories, which runs earlier in startup.
	pub fn new(paths: &AppPaths) -> AppResult<Self>
	{
		// A single session stamp keeps the three channel files visually grouped.
		let session_stamp = Local::now().format("%Y-%m-%d_%H-%M-%S-%3f").to_string();

		// Note: Rust has no named-argument syntax, so these calls are positional;
		// parameter names are kept descriptive in the function signatures instead.
		let action_writer =
			open_channel_file(paths.action_logs_directory(), &session_stamp, "Action")?;
		let debug_writer =
			open_channel_file(paths.debug_logs_directory(), &session_stamp, "Debug")?;
		let network_writer =
			open_channel_file(paths.network_logs_directory(), &session_stamp, "Network")?;

		Ok(Self {
			action_writer: Mutex::new(action_writer),
			debug_writer: Mutex::new(debug_writer),
			network_writer: Mutex::new(network_writer),
		})
	}

	// Record a user action (a click / navigation / form submission).
	pub fn action(&self, message: &str)
	{
		write_line(&self.action_writer, None, message);
	}

	// Record a diagnostic message with the source location of the call site.
	pub fn debug_at(&self, location: &str, message: &str)
	{
		write_line(&self.debug_writer, Some(location), message);
	}

	// Record a network request/response line for the HMRC client.
	pub fn network(&self, message: &str)
	{
		write_line(&self.network_writer, None, message);
	}
}

// Open (create or append) one channel's session file.
fn open_channel_file(
	directory: std::path::PathBuf,
	session_stamp: &str,
	channel: &str,
) -> AppResult<File>
{
	let file_name = format!("{session_stamp}_MyOpenUKTaxApp_{channel}.log");
	let file_path = directory.join(file_name);

	let file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(&file_path)?;

	Ok(file)
}

// Shared writer: prefix each line with a millisecond timestamp and, for the
// debug channel, the originating source location. Errors are reported to stderr
// and otherwise ignored so logging can never take the app down.
fn write_line(writer: &Mutex<File>, location: Option<&str>, message: &str)
{
	let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

	let line = match location
	{
		Some(location) => format!("{timestamp} | {location} | {message}\n"),
		None => format!("{timestamp} | {message}\n"),
	};

	// Lock, append, flush. A poisoned lock or IO failure is logged to stderr.
	match writer.lock()
	{
		Ok(mut file) =>
		{
			if let Err(error) = file.write_all(line.as_bytes())
			{
				eprintln!("log write failed: {error}");
			}
		}
		Err(error) => eprintln!("log lock poisoned: {error}"),
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
