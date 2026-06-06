// Application entry point and wiring.
//
// `run` builds every backend subsystem (portable paths, settings, logging,
// database, optional MCP server) into a shared AppState, registers all Tauri
// commands, then launches the window. Any failure during initialisation is
// fatal and reported on stderr before exiting.

mod commands;
mod db;
mod error;
mod hmrc;
mod housekeeping;
mod logging;
mod mcp;
mod paths;
mod settings;
mod state;
mod util;

use crate::db::Database;
use crate::error::AppResult;
use crate::logging::Logger;
use crate::paths::AppPaths;
use crate::settings::Settings;
use crate::state::AppState;
use std::sync::Arc;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run()
{
	// Build all infrastructure before the window appears.
	let app_state = match initialize()
	{
		Ok(state) => state,
		Err(error) =>
		{
			eprintln!("fatal: failed to initialize MyOpenUKTaxApp: {error}");
			std::process::exit(1);
		}
	};

	// Start the embedded MCP server if it is enabled in settings.
	{
		let (enabled, port) = match app_state.settings.lock()
		{
			Ok(settings) => (settings.mcp_server_enabled, settings.mcp_server_port),
			Err(_) => (false, 0),
		};
		if enabled
		{
			mcp::start(app_state.database.clone(), app_state.logger.clone(), port);
		}
	}

	app_state.logger.action("application started");

	// Launch Tauri with the shared state and the full command surface.
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.manage(app_state)
		.invoke_handler(tauri::generate_handler![
			commands::subcategories::list_subcategories,
			commands::subcategories::create_subcategory,
			commands::subcategories::update_subcategory,
			commands::subcategories::delete_subcategory,
			commands::events::list_events,
			commands::events::get_event,
			commands::events::create_event,
			commands::events::delete_event,
			commands::mappings::list_category_mappings,
			commands::mappings::set_category_mapping,
			commands::mappings::delete_category_mapping,
			commands::dashboard::get_dashboard_summary,
			commands::settings_cmds::get_settings,
			commands::settings_cmds::update_settings,
			commands::hmrc_cmds::list_hmrc_categories,
			commands::hmrc_cmds::list_hmrc_submissions,
			commands::hmrc_cmds::hmrc_status,
			commands::hmrc_cmds::hmrc_authorize_url,
			commands::hmrc_cmds::hmrc_hello_world,
			commands::hmrc_cmds::hmrc_exchange_code,
			commands::hmrc_cmds::hmrc_refresh_token,
			commands::hmrc_cmds::hmrc_submit_period,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}

// Construct the shared application state, creating files/directories as needed.
fn initialize() -> AppResult<AppState>
{
	// Discover where we are and make sure the portable directory tree exists.
	let paths = AppPaths::discover()?;
	paths.ensure_directories()?;

	// Load (or create) settings before anything that depends on them.
	let mut settings = Settings::load_or_create(&paths)?;

	// On first run, mint a stable device id for the HMRC fraud headers.
	if settings.device_id.is_empty()
	{
		settings.device_id = util::random_hex_token(32);
		settings.save(&paths)?;
	}

	// Prune stale logs before opening this session's log files.
	logging::prune_old_logs(&paths, settings.logs_pruned_after_days)?;
	let logger = Arc::new(Logger::new(&paths)?);

	// Open and migrate the database with the configured backup/retention tuning.
	let database = Database::open(
		&paths,
		settings.backup_min_interval_seconds,
		settings.backups_pruned_after_days,
	)?;

	Ok(AppState {
		paths,
		settings: Mutex::new(settings),
		logger,
		database: Arc::new(Mutex::new(database)),
	})
}
