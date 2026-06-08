// Application settings persisted to the exe-adjacent `MyOpenUKTaxApp.settings.json`.
//
// The file is created with sensible defaults the first time the app runs. Every
// field carries a serde default so that older settings files keep loading after
// new options are introduced. HMRC credentials live here too (the file is
// git-ignored); a future hardening pass can move secrets to an OS keystore, but
// the portable single-folder requirement makes a local file the pragmatic store.

use crate::error::AppError;
use crate::error::AppResult;
use crate::paths::AppPaths;
use serde::Deserialize;
use serde::Serialize;

// Allowed values, validated on save so the UI cannot persist nonsense.
pub const ALLOWED_THEMES: [&str; 3] = ["system", "light", "dark"];
pub const ALLOWED_FONT_SIZES: [&str; 9] = [
	"xxx-small",
	"xx-small",
	"x-small",
	"small",
	"medium",
	"large",
	"x-large",
	"xx-large",
	"xxx-large",
];
pub const ALLOWED_HMRC_ENVIRONMENTS: [&str; 2] = ["sandbox", "production"];

// Default retention windows come straight from the overview document.
fn default_backups_pruned_after_days() -> u32
{
	1200
}

fn default_logs_pruned_after_days() -> u32
{
	2200
}

// Smart-backup debounce: at most one automatic backup per this many seconds, so
// a batch of writes does not produce hundreds of near-identical backup files.
fn default_backup_min_interval_seconds() -> u64
{
	300
}

fn default_theme() -> String
{
	"system".to_string()
}

fn default_font_size() -> String
{
	"medium".to_string()
}

fn default_mcp_server_enabled() -> bool
{
	true
}

fn default_mcp_server_port() -> u16
{
	8765
}

fn default_auto_check_for_updates() -> bool
{
	true
}

fn default_auto_update() -> bool
{
	false
}

fn default_hmrc_environment() -> String
{
	"sandbox".to_string()
}

fn default_oauth_redirect_ports() -> Vec<u16>
{
	// A small set of loopback ports registered with HMRC; the app uses the first
	// free one at authorise time, so a collision needs all of them busy at once.
	vec![8350, 8351, 8352, 8353, 8354]
}

// HMRC Making-Tax-Digital connection settings. Credentials are supplied by the
// user after registering an application on the HMRC Developer Hub; they are
// never hardcoded in the source.
//
// Default is implemented by hand (not derived) because a derived Default would
// give `environment` an empty string, which fails settings validation on the
// very first run; the serde field defaults only apply during deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrcSettings
{
	#[serde(default = "default_hmrc_environment")]
	pub environment: String,

	#[serde(default)]
	pub client_id: String,

	#[serde(default)]
	pub client_secret: String,

	// Loopback ports the local OAuth redirect listener may use; the first free one
	// is chosen at authorise time. Register the matching redirect URIs with HMRC.
	#[serde(default = "default_oauth_redirect_ports")]
	pub oauth_redirect_ports: Vec<u16>,

	// The taxpayer's National Insurance number and MTD business id, needed to
	// address the per-business income/expense endpoints.
	#[serde(default)]
	pub national_insurance_number: String,

	#[serde(default)]
	pub business_id: String,

	// OAuth tokens obtained through the authorisation-code flow. Persisted so the
	// user does not have to re-authorise on every launch.
	#[serde(default)]
	pub access_token: String,

	#[serde(default)]
	pub refresh_token: String,

	#[serde(default)]
	pub token_expires_at_epoch_seconds: i64,

	// Sandbox-only: value sent as the `Gov-Test-Scenario` header to select a
	// stubbed HMRC response. Ignored in production.
	#[serde(default)]
	pub gov_test_scenario: String,
}

// Sensible first-run defaults: sandbox environment and the default redirect URI,
// everything else empty until the user fills it in on the Settings screen.
impl Default for HmrcSettings
{
	fn default() -> Self
	{
		Self {
			environment: default_hmrc_environment(),
			client_id: String::new(),
			client_secret: String::new(),
			oauth_redirect_ports: default_oauth_redirect_ports(),
			national_insurance_number: String::new(),
			business_id: String::new(),
			access_token: String::new(),
			refresh_token: String::new(),
			token_expires_at_epoch_seconds: 0,
			gov_test_scenario: String::new(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings
{
	// Stable per-installation identifier sent in the HMRC fraud-prevention
	// `Gov-Client-Device-ID` header. Generated on first run if empty.
	#[serde(default)]
	pub device_id: String,

	#[serde(default = "default_theme")]
	pub theme: String,

	#[serde(default = "default_font_size")]
	pub font_size: String,

	#[serde(default = "default_backups_pruned_after_days")]
	pub backups_pruned_after_days: u32,

	#[serde(default = "default_logs_pruned_after_days")]
	pub logs_pruned_after_days: u32,

	#[serde(default = "default_backup_min_interval_seconds")]
	pub backup_min_interval_seconds: u64,

	#[serde(default = "default_mcp_server_enabled")]
	pub mcp_server_enabled: bool,

	#[serde(default = "default_mcp_server_port")]
	pub mcp_server_port: u16,

	#[serde(default = "default_auto_check_for_updates")]
	pub auto_check_for_updates: bool,

	#[serde(default = "default_auto_update")]
	pub auto_update: bool,

	#[serde(default)]
	pub hmrc: HmrcSettings,
}

// The all-defaults settings used for a brand-new installation.
impl Default for Settings
{
	fn default() -> Self
	{
		Self {
			device_id: String::new(),
			theme: default_theme(),
			font_size: default_font_size(),
			backups_pruned_after_days: default_backups_pruned_after_days(),
			logs_pruned_after_days: default_logs_pruned_after_days(),
			backup_min_interval_seconds: default_backup_min_interval_seconds(),
			mcp_server_enabled: default_mcp_server_enabled(),
			mcp_server_port: default_mcp_server_port(),
			auto_check_for_updates: default_auto_check_for_updates(),
			auto_update: default_auto_update(),
			hmrc: HmrcSettings::default(),
		}
	}
}

impl Settings
{
	// Load settings from disk, creating the file with defaults if it is missing.
	pub fn load_or_create(paths: &AppPaths) -> AppResult<Self>
	{
		let settings_file = paths.settings_file();

		// First run: write a defaults file so the user has something to edit.
		if !settings_file.exists()
		{
			let defaults = Settings::default();
			defaults.save(paths)?;
			return Ok(defaults);
		}

		// Otherwise parse the existing file; serde defaults fill any missing keys.
		let raw = std::fs::read_to_string(&settings_file)?;
		let parsed: Settings = serde_json::from_str(&raw)?;
		Ok(parsed)
	}

	// Persist settings atomically: write to a temp file then rename, so a crash
	// mid-write cannot leave a truncated/corrupt settings file behind.
	pub fn save(&self, paths: &AppPaths) -> AppResult<()>
	{
		self.validate()?;

		let settings_file = paths.settings_file();
		let temporary_file = settings_file.with_extension("json.tmp");

		let serialized = serde_json::to_string_pretty(self)?;
		std::fs::write(&temporary_file, serialized)?;
		std::fs::rename(&temporary_file, &settings_file)?;

		Ok(())
	}

	// Reject values outside the allowed sets so the persisted file stays valid.
	pub fn validate(&self) -> AppResult<()>
	{
		if !ALLOWED_THEMES.contains(&self.theme.as_str())
		{
			return Err(AppError::Validation(format!(
				"theme '{}' is not one of {:?}",
				self.theme, ALLOWED_THEMES
			)));
		}

		if !ALLOWED_FONT_SIZES.contains(&self.font_size.as_str())
		{
			return Err(AppError::Validation(format!(
				"font size '{}' is not one of {:?}",
				self.font_size, ALLOWED_FONT_SIZES
			)));
		}

		if !ALLOWED_HMRC_ENVIRONMENTS.contains(&self.hmrc.environment.as_str())
		{
			return Err(AppError::Validation(format!(
				"HMRC environment '{}' is not one of {:?}",
				self.hmrc.environment, ALLOWED_HMRC_ENVIRONMENTS
			)));
		}

		Ok(())
	}
}
