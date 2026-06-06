// Version bump script run by CI on every build.
//
// version.txt is the single source of truth. The project versioning rule: no
// component may be a double digit — incrementing carries to the next component
// when one would reach 10 (…0.0.9 -> 0.1.0, …0.9.9 -> 1.0.0). The new version is
// written back to version.txt and synced into package.json, tauri.conf.json and
// Cargo.toml so the built artifacts all carry the same number.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const script_directory = dirname(fileURLToPath(import.meta.url));
const repo_root = join(script_directory, "..");

// Increment the dot-separated version with single-digit carry.
function bump_version(version)
{
	const parts = version.trim().split(".").map((part) => parseInt(part, 10));
	if (parts.some((part) => Number.isNaN(part)))
	{
		throw new Error(`version.txt is not a valid dotted version: "${version}"`);
	}

	let index = parts.length - 1;
	parts[index] += 1;
	// Carry left while a component reaches 10, keeping every component single-digit.
	while (index > 0 && parts[index] > 9)
	{
		parts[index] = 0;
		parts[index - 1] += 1;
		index -= 1;
	}

	return parts.join(".");
}

// Replace only the first `version = "..."` line (the [package] version) in Cargo.toml.
function update_cargo_version(cargo_text, new_version)
{
	let replaced = false;
	return cargo_text
		.split("\n")
		.map((line) =>
		{
			if (!replaced && /^version\s*=\s*".*"/.test(line))
			{
				replaced = true;
				return `version = "${new_version}"`;
			}
			return line;
		})
		.join("\n");
}

const version_file = join(repo_root, "version.txt");
const current_version = readFileSync(version_file, "utf8");
const next_version = bump_version(current_version);

// version.txt
writeFileSync(version_file, `${next_version}\n`, "utf8");

// package.json
const package_path = join(repo_root, "package.json");
const package_json = JSON.parse(readFileSync(package_path, "utf8"));
package_json.version = next_version;
writeFileSync(package_path, `${JSON.stringify(package_json, null, "\t")}\n`, "utf8");

// src-tauri/tauri.conf.json
const tauri_conf_path = join(repo_root, "src-tauri", "tauri.conf.json");
const tauri_conf = JSON.parse(readFileSync(tauri_conf_path, "utf8"));
tauri_conf.version = next_version;
writeFileSync(tauri_conf_path, `${JSON.stringify(tauri_conf, null, "\t")}\n`, "utf8");

// src-tauri/Cargo.toml
const cargo_path = join(repo_root, "src-tauri", "Cargo.toml");
const cargo_text = readFileSync(cargo_path, "utf8");
writeFileSync(cargo_path, update_cargo_version(cargo_text, next_version), "utf8");

// Emit the new version for the workflow (and a friendly log line).
console.log(next_version);
if (process.env.GITHUB_OUTPUT)
{
	writeFileSync(process.env.GITHUB_OUTPUT, `version=${next_version}\n`, { flag: "a" });
}
