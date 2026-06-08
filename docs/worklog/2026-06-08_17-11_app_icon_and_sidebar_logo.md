# 2026-06-08 — Application icon and sidebar Dashboard logo

Continues from `2026-06-08_12-33_hmrc_connect_validation_and_api_versions.md`.
Short session: adopt the new brand logo (prepared by the user in `gfx/logo/`)
as both the OS application icon and the in-app sidebar Dashboard icon.

## Source assets

The user supplied, in `gfx/logo/`:

- `logo.svg` — raw, non-shadow vector source.
- `logo.png` — 1024×1024, clean (no shadow), transparent.
- `logo_shadow.png` — 1268×1268, with a drop shadow.
- `logo.ico`, `logo.jsx` — additional exports (not consumed directly).

## OS / application icon

- Regenerated the full Tauri icon set with `npx tauri icon gfx/logo/logo.png`,
  overwriting `src-tauri/icons/` (`icon.ico`, `icon.icns`, `icon.png`,
  `32x32.png`, `128x128.png`, `128x128@2x.png`, the Windows-store `Square*Logo.png`
  sizes, plus iOS/Android assets).
- Used the **clean `logo.png`**, not `logo_shadow.png`: a baked-in drop shadow
  gets cropped and looks muddy at small OS sizes (16/32 px), and the OS renders
  its own shadow anyway.
- No `tauri.conf.json` change needed — its `bundle.icon` array already points at
  these filenames.

### Forcing the icon into the dev binary

The Windows taskbar/window icon is embedded into the `.exe` at build time by
`tauri-build` (via `build.rs`). Cargo does **not** treat the icon files as build
inputs, so after regenerating them the first `tauri dev` relaunch reported
`Finished in 1.02s` (no recompile) and the taskbar still showed the **old default
Tauri icon**.

Fix: bump `src-tauri/build.rs`'s timestamp to invalidate Cargo's fingerprint,
then relaunch so the build script re-runs and re-embeds the new `.ico`. After a
real recompile (≈24 s) the new icon appeared on the taskbar. **Remember this
step whenever the logo changes again** — editing icon files alone won't update
the embedded binary icon.

## In-app sidebar Dashboard icon

- `src/components/layout/sidebar.tsx`: the Dashboard nav entry now renders the
  logo image instead of the Material `dashboard` glyph. Other sections keep their
  Material icons. The logo sits in the same 24 px (`h-6 w-6`) box as the rest, with
  `object-contain` and `aria-hidden` (the label provides the accessible name).
- Used **`logo_shadow.png`** here (the shadowed variant): against the card
  background in a UI context the shadow reads as a proper logo, and unlike OS
  icons it isn't shrunk to 16 px.
- Made the asset available to the frontend by copying it to
  `public/logo_shadow.png` (Vite serves `public/` at the web root, so it loads
  from `/logo_shadow.png`).

## Verification

- `npm run typecheck` — clean.
- Ran `npm run tauri dev` (after a forced recompile) and confirmed visually:
  sidebar Dashboard logo and the Windows taskbar icon both show the new brand
  logo. User confirmed "The new logo looks good now."

## State at end of session

- All changes are **uncommitted** working-tree edits (version still 0.0.5); the
  user commits to GitHub manually.
- Touched files: `src-tauri/icons/*` (regenerated), `src-tauri/build.rs`
  (timestamp only — content unchanged), `src/components/layout/sidebar.tsx`,
  `public/logo_shadow.png` (new). HMRC open items from the previous worklog are
  unchanged and still pending.
