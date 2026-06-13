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
mod runmode;
mod settings;
mod state;
mod util;
mod window_state;

use crate::db::Database;
use crate::error::AppResult;
use crate::logging::Logger;
use crate::paths::AppPaths;
use crate::settings::Settings;
use crate::state::AppState;
use crate::window_state::WindowState;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::Manager;

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
		// Restore the saved window geometry/mode before the (initially hidden)
		// window is shown, so there is no flash of a default-sized window.
		.setup(|app| {
			if let Some(window) = app.get_webview_window("main")
			{
				let app_state = app.state::<AppState>();
				window_state::restore_on_launch(&window, &app_state);
			}
			// Seed the session baseline (and write the initial state file) from the
			// settled launch geometry, so the user's very first move/resize is
			// recognised as a customisation rather than mistaken for the baseline.
			window_state::schedule_save(app.app_handle().clone());
			Ok(())
		})
		// Track moves/resizes/maximize/minimize (debounced) and persist on close.
		.on_window_event(|window, event| {
			match event
			{
				tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) =>
				{
					window_state::schedule_save(window.app_handle().clone());
				}
				tauri::WindowEvent::CloseRequested { .. } =>
				{
					window_state::save_now(window.app_handle());
				}
				_ =>
				{}
			}
		})
		.invoke_handler(tauri::generate_handler![
			commands::subcategories::list_subcategories,
			commands::subcategories::create_subcategory,
			commands::subcategories::update_subcategory,
			commands::subcategories::delete_subcategory,
			commands::events::list_events,
			commands::events::get_event,
			commands::events::create_event,
			commands::events::delete_event,
			commands::events::last_used_subcategories,
			commands::mappings::list_category_mappings,
			commands::mappings::set_category_mapping,
			commands::mappings::delete_category_mapping,
			commands::dashboard::get_dashboard_summary,
			commands::settings_cmds::get_settings,
			commands::settings_cmds::update_settings,
			commands::settings_cmds::restart_app,
			commands::settings_cmds::open_data_directory,
			commands::settings_cmds::open_logs_directory,
			commands::app_cmds::app_info,
			commands::app_cmds::check_latest_version,
			commands::mode_cmds::get_run_mode,
			commands::mode_cmds::set_run_mode,
			commands::hmrc_cmds::list_hmrc_categories,
			commands::hmrc_cmds::list_hmrc_submissions,
			commands::hmrc_cmds::hmrc_list_businesses,
			commands::hmrc_cmds::hmrc_set_business_id,
			commands::hmrc_cmds::hmrc_status,
			commands::hmrc_cmds::hmrc_redirect_uris,
			commands::hmrc_cmds::hmrc_authorize,
			commands::hmrc_cmds::hmrc_hello_world,
			commands::hmrc_cmds::hmrc_refresh_token,
			commands::hmrc_cmds::hmrc_submit_period,
			commands::hmrc_cmds::hmrc_get_business_details,
			commands::hmrc_cmds::hmrc_get_obligations_quarterly,
			commands::hmrc_cmds::hmrc_get_obligations_final_declaration,
			commands::hmrc_cmds::hmrc_get_cumulative,
			commands::hmrc_cmds::hmrc_get_annual,
			commands::hmrc_cmds::hmrc_get_period_summaries,
			commands::hmrc_cmds::hmrc_get_biss,
			commands::hmrc_cmds::hmrc_get_calculations,
			commands::hmrc_cmds::hmrc_get_sa_account,
			commands::hmrc_cmds::hmrc_setup_test_data,
			commands::hmrc_cmds::hmrc_test_data_seeded_at,
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

	// Prune stale logs and clear out any zero-byte logs left by older versions
	// before this session's (lazily-created) log files begin.
	logging::prune_old_logs(&paths, settings.logs_pruned_after_days)?;
	logging::remove_empty_logs(&paths)?;
	let logger = Arc::new(Logger::new(&paths)?);

	// Load the run mode (Sandbox/Production) so the DB opens on the right schema.
	let run_mode = runmode::load(&paths);

	// Open and migrate the database (both schemas) with the configured tuning.
	let database = Database::open(
		&paths,
		settings.backup_min_interval_seconds,
		settings.backups_pruned_after_days,
		run_mode.schema(),
	)?;

	// Load any saved window geometry/mode before paths is moved into the state.
	let window_state = WindowState::load_or_default(&paths);

	Ok(AppState {
		paths,
		settings: Mutex::new(settings),
		logger,
		database: Arc::new(Mutex::new(database)),
		run_mode: Mutex::new(run_mode),
		window_state: Mutex::new(window_state),
		window_baseline: Mutex::new(None),
		window_save_generation: std::sync::atomic::AtomicU64::new(0),
	})
}
