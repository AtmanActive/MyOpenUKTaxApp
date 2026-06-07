# 2026-06-07 (afternoon) — Settings UX, Updates/About, recent-events, logging, filter cues, and HMRC OAuth

Continues from `2026-06-07_09-58_github_ci_release_pipeline.md`. A long live-refinement
session: the app ran under `npm run tauri dev` throughout, so most changes were
verified live (frontend via HMR, backend via the dev watcher's recompile+relaunch).
Each change was checked with `cargo check` and/or `npm run typecheck`.

## Settings screen — auto-save, no Save button
- Removed the Save button. Every control auto-saves on change: selects and the
  checkbox save immediately; text/number inputs are debounced (500ms). A small
  "Saving… / All changes saved" indicator was added.
- Live-apply everywhere possible: theme/font re-apply via the settings cache;
  HMRC fields are read per-call; backup retention is pushed to the DB handle.
- Log retention now applies live: `update_settings` re-prunes logs when
  `logs_pruned_after_days` changes (previously startup-only).
- The MCP server is the only startup-only setting, so when its enable/port differ
  from the values the app launched with, an inline **Restart now** button appears.
  New `restart_app` command (`AppHandle::restart`).

## New Settings sections
- **Updates**: current version; auto-check + auto-update toggles (persisted);
  **Check now** (queries the GitHub Releases API) showing the latest version /
  up-to-date / none; **Update now** opens the latest release to download
  (lightweight, unsigned-build approach — no in-place self-update yet). New
  `check_latest_version` command; settings `auto_check_for_updates`/`auto_update`.
- **About**: app name, author, clickable homepage (opens browser), licence. New
  `app_info` command; `homepage` added to Cargo.toml.

## Settings topbar
- Added a right-justified **✕** that returns to the Dashboard (same as the
  sidebar Dashboard entry), shown only on Settings.

## Add Event
- **Category auto-select**: never shows an unselected dropdown. Picks the
  last-used category per kind (derived from the most recent event of that kind:
  new `last_used_subcategories` command), else the first available; one category
  ⇒ auto-selected. Applies on load, on Income/Expense tab switch, and after save.
  Fixed a timing bug where the just-saved category wasn't pre-selected (now taken
  directly from the saved event rather than waiting on an async refetch).
- **Recently added (this session)**: an in-memory, non-persisted table under the
  form showing the last 3 events (newest first, income/expenses intermingled,
  amount colour-coded), with opacity fading by age (100/75/50%). Label later
  removed per request.
- **Enter to save**: pressing Enter in the Note field submits the event.

## Theming / logging / file access
- Native date-picker calendar now follows the theme (added CSS `color-scheme`
  light/dark), fixing the light calendar in dark mode.
- **Open data directory** / **Open logs directory** buttons in the Data & logs
  card (`open_data_directory`/`open_logs_directory`, via the opener plugin).
- **No more zero-byte logs**: log files are created lazily on first write, so an
  unused channel (often Network) leaves no file; a startup `remove_empty_logs`
  sweep clears empties left by older builds.

## Topbar filter cues
- When a filter is active (search term or date range): a flashing filter icon, a
  circled-✕ that clears the whole filter (new `clear_filter` store action), a
  dark-red topbar, and a " (filtered)" suffix on the title.

## Incident: blank screen (dev only)
- After editing the zustand store module, the running dev webview went blank.
  Cause was HMR/fast-refresh state corruption from hot-swapping the shared store,
  not a code bug (the production build was clean). A clean dev-server restart
  fixed it. Takeaway: edits to shared modules may need a full reload.

## HMRC — Business ID lookup
- Clarified that **Business ID ≠ UTR** (also Application ID = the OAuth Client ID).
  Updated the Business ID field hint.
- **Fetch my businesses**: calls HMRC *List All Businesses*
  (`GET /individuals/business/details/{nino}/list`), parses `listOfBusinesses`,
  and lets the user pick (auto-selects when there's exactly one). New
  `hmrc_list_businesses` command + `HmrcClient::list_businesses`.

## HMRC — one-click OAuth via a local loopback listener
- Replaced the manual "open URL → copy code → paste → exchange" flow with a
  single **Authorise with HMRC** button. New `hmrc_authorize` command:
  binds a free loopback port, opens HMRC sign-in, runs a local listener that
  captures the redirect, verifies the CSRF `state`, exchanges the code, and
  stores the tokens. Removed `hmrc_authorize_url` and `hmrc_exchange_code`.
- **Port robustness**: replaced the single `redirect_uri` setting with a list,
  `oauth_redirect_ports` (default 8350–8354); the app binds the first free one.
  Settings shows the exact redirect URIs to register on the Developer Hub.
- **Host**: must be `localhost` (HMRC rejects `127.0.0.1` with 403). The listener
  binds the IPv4 loopback.
- **Dual-stack fix**: Waterfox resolved `localhost` to IPv6 `::1` and did not fall
  back, so the redirect couldn't connect. The listener now binds **both**
  `127.0.0.1` and `[::1]` (a thread per address; first valid callback wins).
  Confirmed working end-to-end against the HMRC sandbox.

## State at end of session
- All changes are **uncommitted** working-tree edits (version still 0.0.5); the
  user will commit to GitHub manually. Releases are manual-only (`workflow_dispatch`).
- A `npm run tauri dev` instance and the app window were left running during the
  session.

## Open items / next steps
- True in-place self-update (signed updater) remains a future step.
- Optional: make `oauth_redirect_ports` editable in the UI; a checks-only CI on push.
- The HMRC submission body still needs tightening to the exact MTD schema/version.
- Tests (the standards' test-first goal) as features settle.
