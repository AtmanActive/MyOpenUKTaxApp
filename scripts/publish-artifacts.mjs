// Renames the bundles that tauri-action produced so each carries a platform and
// architecture token, then uploads them to the current GitHub release. Run once
// per matrix OS job. All inputs arrive via environment variables set by the
// workflow; the build paths come from tauri-action's `artifactPaths` output, so
// this never has to hardcode Tauri's per-platform output directories.

import { execSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { extname, join } from "node:path";

// Read a required environment variable or fail loudly.
function require_env(name)
{
	const value = process.env[name];
	if (!value)
	{
		console.error(`missing required environment variable: ${name}`);
		process.exit(1);
	}
	return value;
}

const version = require_env("VERSION");
const os_label = require_env("OS_LABEL");
const arch = require_env("ARCH");
const tag = require_env("TAG");

// artifactPaths is a JSON array string; an empty value means "nothing built".
let artifact_paths_json = process.env.ARTIFACT_PATHS;
if (!artifact_paths_json)
{
	artifact_paths_json = "[]";
}

let artifact_paths;
try
{
	artifact_paths = JSON.parse(artifact_paths_json);
}
catch
{
	console.error("ARTIFACT_PATHS is not valid JSON");
	process.exit(1);
}

// Build the canonical, platform-tagged asset name for one bundle, or null to
// skip things we do not distribute (the macOS .app directory, updater .sig, …).
function canonical_name(source_path)
{
	const stem = `MyOpenUKTaxApp_${version}_${os_label}_${arch}`;
	switch (extname(source_path).toLowerCase())
	{
		case ".exe":
			return `${stem}_setup.exe`;
		case ".dmg":
			return `${stem}.dmg`;
		case ".deb":
			return `${stem}.deb`;
		case ".appimage":
			return `${stem}.AppImage`;
		default:
			return null;
	}
}

const output_directory = "release-assets";
mkdirSync(output_directory, { recursive: true });

// Copy each distributable bundle to its tagged name.
const uploads = [];
for (const source_path of artifact_paths)
{
	if (!existsSync(source_path))
	{
		continue;
	}
	const name = canonical_name(source_path);
	if (!name)
	{
		continue;
	}
	const destination = join(output_directory, name);
	copyFileSync(source_path, destination);
	uploads.push(destination);
}

if (uploads.length === 0)
{
	console.log("No distributable artifacts matched; nothing to upload.");
	process.exit(0);
}

console.log("Uploading artifacts:\n" + uploads.join("\n"));

// File names contain no spaces, but quote defensively for both shells.
const quoted = uploads.map((file) => `"${file}"`).join(" ");
execSync(`gh release upload ${tag} ${quoted} --clobber`, { stdio: "inherit" });
