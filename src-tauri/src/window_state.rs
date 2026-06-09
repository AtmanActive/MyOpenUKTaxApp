// Window geometry + display-mode persistence.
//
// The app remembers, between runs, whether it was minimized / maximized / in the
// normal window state and — once the user has manually moved or resized it — the
// exact position and size to restore. Like everything else in this portable app,
// the state lives in an exe-adjacent JSON file (MyOpenUKTaxApp.window.json),
// never in an OS per-user directory.
//
// Until the user first moves or resizes the window, no geometry is pinned and the
// initial placement is left to the operating system (a best-effort guess). The
// `customized` flag is sticky: once set, the window always restores to its last
// normal geometry.

use crate::error::AppResult;
use crate::paths::AppPaths;
use crate::state::AppState;
use serde::Deserialize;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;
use tauri::PhysicalPosition;
use tauri::PhysicalSize;
use tauri::WebviewWindow;

// How long to wait for window move/resize/maximize events to settle before
// sampling the final geometry. Windows emits transient events mid-maximize where
// the rectangle is already maximized but `is_maximized()` is not yet true;
// sampling only after the dust settles avoids capturing those as a normal-window
// move and reading a reliable maximized/minimized flag.
const SETTLE_DELAY: Duration = Duration::from_millis(250);

// The window's display mode at the moment state was last saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode
{
	Normal,
	Maximized,
	Minimized,
}

impl Default for WindowMode
{
	fn default() -> Self
	{
		WindowMode::Normal
	}
}

// A window rectangle in physical pixels: outer position + inner size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry
{
	pub x: i32,
	pub y: i32,
	pub width: u32,
	pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState
{
	// Display mode at last save; reproduced on the next launch.
	#[serde(default)]
	pub mode: WindowMode,

	// Last *normal* (neither maximized nor minimized) geometry. Only applied on
	// launch when `customized` is true; otherwise the OS picks the placement.
	#[serde(default)]
	pub x: i32,
	#[serde(default)]
	pub y: i32,
	#[serde(default)]
	pub width: u32,
	#[serde(default)]
	pub height: u32,

	// True once the user has manually moved or resized the window at least once
	// (sticky across runs). Until then placement is left to the OS.
	#[serde(default)]
	pub customized: bool,
}

impl Default for WindowState
{
	fn default() -> Self
	{
		Self { mode: WindowMode::Normal, x: 0, y: 0, width: 0, height: 0, customized: false }
	}
}

impl WindowState
{
	// Load saved window state, returning defaults if the file is missing or
	// unreadable — a corrupt file must never stop the window from opening.
	pub fn load_or_default(paths: &AppPaths) -> Self
	{
		match std::fs::read_to_string(paths.window_state_file())
		{
			Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
			Err(_) => Self::default(),
		}
	}

	// Persist atomically (temp file + rename), the same way settings are written,
	// so a crash mid-write cannot leave a truncated file behind.
	pub fn save(&self, paths: &AppPaths) -> AppResult<()>
	{
		let file = paths.window_state_file();
		let temporary_file = file.with_extension("json.tmp");
		let serialized = serde_json::to_string_pretty(self)?;
		std::fs::write(&temporary_file, serialized)?;
		std::fs::rename(&temporary_file, &file)?;
		Ok(())
	}

	fn geometry(&self) -> Geometry
	{
		Geometry { x: self.x, y: self.y, width: self.width, height: self.height }
	}

	fn set_geometry(&mut self, geometry: Geometry)
	{
		self.x = geometry.x;
		self.y = geometry.y;
		self.width = geometry.width;
		self.height = geometry.height;
	}
}

// Read the live outer position + inner size of the window.
fn read_geometry(window: &WebviewWindow) -> Option<Geometry>
{
	let position = window.outer_position().ok()?;
	let size = window.inner_size().ok()?;
	Some(Geometry { x: position.x, y: position.y, width: size.width, height: size.height })
}

// Restore saved geometry/mode, make the window visible, and seed the session
// baseline. Called once from the Tauri `setup` hook while the window is still
// hidden, so there is no flash of a default window before it maximizes/minimizes.
pub fn restore_on_launch(window: &WebviewWindow, app_state: &AppState)
{
	let state = app_state
		.window_state
		.lock()
		.map(|state| state.clone())
		.unwrap_or_default();

	// Pin exact geometry only if the user customised it and it still lands on a
	// connected monitor (so a window saved on another machine cannot open
	// off-screen).
	if state.customized && state.width > 0 && state.height > 0 && geometry_on_screen(window, state.geometry())
	{
		let _ = window.set_size(PhysicalSize::new(state.width, state.height));
		let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
	}

	if state.mode == WindowMode::Maximized
	{
		let _ = window.maximize();
	}

	// Reveal the window (it starts hidden — see tauri.conf.json `visible: false`).
	let _ = window.show();

	// Minimise only after showing, so the app still gets a taskbar button.
	if state.mode == WindowMode::Minimized
	{
		let _ = window.minimize();
	}

	// The session baseline is intentionally left unset here: the first *settled*
	// normal-mode observation establishes it (see `record`). That way startup
	// layout jitter all collapses to one reference point and is never mistaken for
	// a user move/resize.
}

// True if the rectangle overlaps any currently-connected monitor. Guards against
// restoring onto a monitor that no longer exists (different machine / unplugged
// display); on any uncertainty it returns true rather than block the restore.
fn geometry_on_screen(window: &WebviewWindow, geometry: Geometry) -> bool
{
	let monitors = match window.available_monitors()
	{
		Ok(monitors) if !monitors.is_empty() => monitors,
		_ => return true,
	};

	let (x, y, w, h) = (geometry.x, geometry.y, geometry.width as i32, geometry.height as i32);
	monitors.iter().any(|monitor| {
		let origin = monitor.position();
		let size = monitor.size();
		let (mx, my, mw, mh) = (origin.x, origin.y, size.width as i32, size.height as i32);
		// Standard rectangle-overlap test.
		x < mx + mw && x + w > mx && y < my + mh && y + h > my
	})
}

// Refresh the in-memory state from the live (settled) window. Captures the
// display mode always, and the geometry only while normal — minimized and
// maximized windows report rectangles we must not save as the restore geometry.
fn record(window: &WebviewWindow, app_state: &AppState)
{
	let minimized = window.is_minimized().unwrap_or(false);
	let maximized = window.is_maximized().unwrap_or(false);
	let mode = if minimized
	{
		WindowMode::Minimized
	}
	else if maximized
	{
		WindowMode::Maximized
	}
	else
	{
		WindowMode::Normal
	};

	let mut state = match app_state.window_state.lock()
	{
		Ok(state) => state,
		Err(_) => return,
	};
	state.mode = mode;

	if mode == WindowMode::Normal
	{
		if let Some(geometry) = read_geometry(window)
		{
			if let Ok(mut baseline) = app_state.window_baseline.lock()
			{
				match *baseline
				{
					// First settled normal geometry of the session is the reference
					// point; it is not, by itself, a user customisation.
					None => *baseline = Some(geometry),
					// Any later deviation from it means the user moved/resized.
					Some(reference) =>
					{
						if geometry != reference
						{
							state.customized = true;
						}
					}
				}
			}
			state.set_geometry(geometry);
		}
	}
}

// Write the current in-memory state to disk (best-effort; logs on failure so a
// disk error can never crash the window-event handler).
fn persist(app_state: &AppState)
{
	if let Ok(state) = app_state.window_state.lock()
	{
		if let Err(error) = state.save(&app_state.paths)
		{
			app_state
				.logger
				.debug_at("window_state", &format!("failed to save window state: {error}"));
		}
	}
}

// Debounced save in response to a move/resize/maximize/minimize event. Each call
// bumps a generation counter and spawns a short-lived thread; only the last
// scheduled save in a burst actually samples and writes, so transient mid-gesture
// states are skipped and a drag does not hammer the disk.
pub fn schedule_save(app: AppHandle)
{
	let generation = app
		.state::<AppState>()
		.window_save_generation
		.fetch_add(1, Ordering::SeqCst)
		+ 1;

	std::thread::spawn(move || {
		std::thread::sleep(SETTLE_DELAY);
		let app_state = app.state::<AppState>();
		// A newer event arrived while we waited — let that later save win.
		if app_state.window_save_generation.load(Ordering::SeqCst) != generation
		{
			return;
		}
		if let Some(window) = app.get_webview_window("main")
		{
			record(&window, &app_state);
			persist(&app_state);
		}
	});
}

// Synchronous save for window close, where a debounce thread might not run before
// the process exits. Bumps the generation so any pending debounced save is voided.
pub fn save_now(app: &AppHandle)
{
	let app_state = app.state::<AppState>();
	app_state.window_save_generation.fetch_add(1, Ordering::SeqCst);
	if let Some(window) = app.get_webview_window("main")
	{
		record(&window, &app_state);
		persist(&app_state);
	}
}
