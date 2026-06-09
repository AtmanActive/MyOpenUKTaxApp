# 2026-06-09 — Window state persistence (position / size / minimized / maximized)

Continues from `2026-06-08_17-11_app_icon_and_sidebar_logo.md`. This session adds
remembering and restoring the main window's geometry and display mode between
runs.

## Requirements (as specified)

1. Last **minimized** → start minimized.
2. Last **maximized** → start maximized.
3. Last **normal** (restored) → start in the normal window state.
4. Window **manually resized/moved** → start with those exact coordinates and size.
5. Window **never resized/moved** → start best-effort, letting the OS place it.

## Approach: custom, not the official plugin

Implemented by hand rather than using `tauri-plugin-window-state`, for two
project-specific reasons:

- **Portability.** The official plugin persists to an OS per-user config dir,
  which violates this app's "everything lives next to the .exe" rule. Our state
  is stored in an exe-adjacent `MyOpenUKTaxApp.window.json` (atomic temp-file +
  rename, like the settings file), kept separate from `MyOpenUKTaxApp.settings.json`
  so frequent window writes never touch the file holding HMRC credentials.
- **Minimized restore.** The official plugin does not restore a minimized window
  (requirement #1); the custom version does.

## What was added

- **`src-tauri/src/window_state.rs`** (new):
  - `WindowMode` (`normal` | `maximized` | `minimized`), `Geometry`, and the
    serialisable `WindowState { mode, x, y, width, height, customized }` with
    `load_or_default` / atomic `save`.
  - `restore_on_launch` — applies saved geometry (only if `customized` and still
    on-screen), maximizes if needed, `show()`s the (initially hidden) window, then
    minimizes if needed.
  - `record` — samples the live window: captures `mode` always, and geometry only
    while **normal** (minimized/maximized report rectangles we must not save as the
    restore geometry).
  - `schedule_save` / `save_now` — debounced and synchronous persistence (below).
  - `geometry_on_screen` — rectangle-overlap test against `available_monitors`, so
    a window saved on a different machine/monitor layout can't open off-screen.
- **`src-tauri/src/paths.rs`**: `window_state_file()` → exe-adjacent
  `MyOpenUKTaxApp.window.json`.
- **`src-tauri/src/state.rs`**: `AppState` gains `window_state: Mutex<WindowState>`,
  `window_baseline: Mutex<Option<Geometry>>`, and `window_save_generation: AtomicU64`.
- **`src-tauri/src/lib.rs`**: load saved state in `initialize`; a `setup` hook
  restores it and kicks off one debounced save to seed the baseline; an
  `on_window_event` handler debounces saves on `Resized`/`Moved` and saves
  synchronously on `CloseRequested`.
- **`src-tauri/tauri.conf.json`**: main window `"visible": false` — it starts
  hidden and `restore_on_launch` shows it, so there is no flash of a default-sized
  window before it maximizes/minimizes/repositions.

## "Customized" detection (requirements 4 vs 5)

- A sticky `customized` flag distinguishes "user arranged the window" from "never
  touched". Until it is set, geometry is never pinned and the OS chooses placement.
- It is detected by diffing each settled normal-mode geometry against a per-session
  **baseline**. The first settled normal observation establishes the baseline; any
  later deviation means the user moved/resized → `customized = true` (persisted, so
  it stays true across runs).
- `setup` schedules one debounced save at launch so the baseline is seeded from the
  settled default geometry. Without this, the very first drag would have *become*
  the baseline instead of being detected as a change (since `visible:false` →
  `show()` emits no move/resize event to seed from).

## Bug found and fixed during testing: maximize polluted the saved geometry

First cut saved synchronously on every `Resized`/`Moved`. On Windows, maximizing
emits transient events where the rectangle is already the maximized one but
`is_maximized()` has **not yet flipped to true**. That transient was recorded as a
normal-mode move:

```jsonc
// BUG: after only maximizing (never resizing)
{ "mode": "maximized", "x": -11, "y": -11, "width": 1920, "height": 1000, "customized": true }
```

— wrong `customized`, and the maximized rect leaked into the "normal" geometry.

**Fix:** debounce. Each event bumps an `AtomicU64` generation and spawns a
short-lived thread; only the last save in a 250 ms burst samples and writes, by
which point `is_maximized()`/`is_minimized()` have settled. `CloseRequested` saves
synchronously (a debounce thread might not run before the process exits) and bumps
the generation to void any pending debounced save. Debouncing also stops disk churn
during a drag.

## Verification

`cargo check` clean (no warnings). Ran under `npm run tauri dev` and inspected
`MyOpenUKTaxApp.window.json` (written to `src-tauri/target/debug/` in dev; sits next
to the installed exe in a packaged build):

- Untouched launch → `{ mode: normal, customized: false }` (geometry recorded but
  ignored on restore).
- Maximize only → `{ mode: "maximized", customized: false }`, normal geometry
  preserved (the bug above no longer reproduces).
- Manual resize/move → `{ mode: normal, customized: true }` with the new
  coordinates/size.
- Clean kill + relaunch → window reopened at the exact saved geometry; the file was
  re-read unchanged, confirming restore applied.

Minimized-start and maximized-start use the same verified restore path
(`show()` then `minimize()`, or `maximize()` before `show()`).

## State at end of session

- All changes are **uncommitted** working-tree edits (version 0.0.6); the user
  commits to GitHub manually. Dev app left running.
- HMRC open items from earlier worklogs remain unchanged and still pending.
