// SQLite data layer.
//
// Persistence is split into two parallel schemas — `sandbox` and `production` —
// held in separate files (Data/MyOpenUKTaxApp.{sandbox,production}.db) and ATTACHed
// to a single in-memory control connection. The two schemas have identical tables
// but independent data. The `active_schema` selects which one every query targets;
// switching it (when the run mode changes) is instant, with no reconnection.
//
// Money is stored as integer pence. Migrations run per schema (each attached file
// carries its own PRAGMA user_version). Pre-write backups are "smart" (debounced).

pub mod models;

use crate::error::AppError;
use crate::error::AppResult;
use crate::housekeeping;
use crate::paths::AppPaths;
use chrono::NaiveDate;
use chrono::Utc;
use models::*;
use rusqlite::params;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::time::Duration;
use std::time::Instant;

// Guard against absurd monetary input (one billion pounds), in pence.
const MAX_AMOUNT_PENCE: i64 = 100_000_000_000;

// The two schema names; also the ATTACH aliases and part of the backup filenames.
const SCHEMAS: [&str; 2] = ["sandbox", "production"];

// The default subcategories every new schema is seeded with: (kind, name).
const DEFAULT_SUBCATEGORIES: [(&str, &str); 6] = [
	("income", "Main"),
	("expense", "Phone"),
	("expense", "Internet"),
	("expense", "Utilities"),
	("expense", "Bank"),
	("expense", "Capital"),
];

// The fixed HMRC self-employment income/expense categories defined by the MTD
// Income Tax spec: (code, kind, label). Seeded into each schema so the Category
// Mapping screen works out of the box; the same codes build a period submission.
const DEFAULT_HMRC_CATEGORIES: [(&str, &str, &str); 17] = [
	("turnover", "income", "Turnover (takings, fees, sales or money earned)"),
	("other", "income", "Any other business income"),
	("costOfGoods", "expense", "Cost of goods bought for resale or goods used"),
	("paymentsToSubcontractors", "expense", "Construction industry: payments to subcontractors"),
	("wagesAndStaffCosts", "expense", "Wages, salaries and other staff costs"),
	("carVanTravelExpenses", "expense", "Car, van and travel expenses"),
	("premisesRunningCosts", "expense", "Rent, rates, power and insurance costs"),
	("maintenanceCosts", "expense", "Repairs and maintenance of property and equipment"),
	("adminCosts", "expense", "Phone, fax, stationery and other office costs"),
	("advertisingCosts", "expense", "Advertising and business entrance costs"),
	("businessEntertainmentCosts", "expense", "Business entertainment costs"),
	("interestOnBankOtherLoans", "expense", "Interest on bank and other loans"),
	("financeCharges", "expense", "Bank, credit card and other financial charges"),
	("irrecoverableDebts", "expense", "Irrecoverable debts written off"),
	("professionalFees", "expense", "Accountancy, legal and other professional fees"),
	("depreciation", "expense", "Depreciation and loss or profit on sale of assets"),
	("otherExpenses", "expense", "Other business expenses"),
];

// The v1 schema DDL for one schema (table/index names qualified with the schema;
// REFERENCES stay unqualified so SQLite resolves them within the same schema).
fn schema_v1_ddl(schema: &str) -> String
{
	format!(
		"CREATE TABLE {schema}.subcategories (
			id          INTEGER PRIMARY KEY AUTOINCREMENT,
			kind        TEXT NOT NULL CHECK (kind IN ('income','expense')),
			name        TEXT NOT NULL,
			description TEXT NOT NULL DEFAULT '',
			created_at  TEXT NOT NULL,
			UNIQUE (kind, name)
		);

		CREATE TABLE {schema}.ledger_events (
			id             INTEGER PRIMARY KEY AUTOINCREMENT,
			kind           TEXT NOT NULL CHECK (kind IN ('income','expense')),
			event_date     TEXT NOT NULL,
			subcategory_id INTEGER NOT NULL REFERENCES subcategories(id),
			amount_pence   INTEGER NOT NULL CHECK (amount_pence >= 0),
			note           TEXT NOT NULL DEFAULT '',
			created_at     TEXT NOT NULL
		);
		CREATE INDEX {schema}.idx_ledger_events_date ON ledger_events(event_date);
		CREATE INDEX {schema}.idx_ledger_events_subcategory ON ledger_events(subcategory_id);

		CREATE TABLE {schema}.hmrc_categories (
			id          INTEGER PRIMARY KEY AUTOINCREMENT,
			code        TEXT NOT NULL UNIQUE,
			kind        TEXT NOT NULL,
			label       TEXT NOT NULL,
			description TEXT NOT NULL DEFAULT '',
			updated_at  TEXT NOT NULL
		);

		CREATE TABLE {schema}.category_mappings (
			id               INTEGER PRIMARY KEY AUTOINCREMENT,
			subcategory_id   INTEGER NOT NULL UNIQUE REFERENCES subcategories(id),
			hmrc_category_id INTEGER NOT NULL REFERENCES hmrc_categories(id),
			created_at       TEXT NOT NULL
		);

		CREATE TABLE {schema}.hmrc_submissions (
			id            INTEGER PRIMARY KEY AUTOINCREMENT,
			period_from   TEXT NOT NULL,
			period_to     TEXT NOT NULL,
			submitted_at  TEXT NOT NULL,
			status        TEXT NOT NULL,
			reference     TEXT NOT NULL DEFAULT '',
			request_json  TEXT NOT NULL DEFAULT '',
			response_json TEXT NOT NULL DEFAULT ''
		);"
	)
}

pub struct Database
{
	connection: Connection,
	paths: AppPaths,
	// The schema every query currently targets ("sandbox" | "production").
	active_schema: String,
	// Retention/backup tuning copied from settings; refreshed when settings change.
	backup_min_interval: Duration,
	backups_pruned_after_days: u32,
	// When the most recent automatic backup was taken this session (None = never).
	last_backup_at: Option<Instant>,
}

impl Database
{
	// Open the control connection, ATTACH both schema files, migrate each, seed
	// defaults, and select the starting schema. A pre-existing single-file database
	// (Data/MyOpenUKTaxApp.db) is migrated once into the sandbox schema file.
	pub fn open(
		paths: &AppPaths,
		backup_min_interval_seconds: u64,
		backups_pruned_after_days: u32,
		active_schema: &str,
	) -> AppResult<Self>
	{
		// One-time migration: the legacy single DB becomes the sandbox schema.
		let legacy = paths.database_file();
		let sandbox_file = paths.sandbox_database_file();
		if legacy.exists() && !sandbox_file.exists()
		{
			std::fs::rename(&legacy, &sandbox_file)?;
		}

		// A throwaway in-memory control DB owns the connection; the real data lives
		// in the two attached schema files.
		let connection = Connection::open_in_memory()?;
		connection.execute_batch("PRAGMA foreign_keys = ON;")?;

		// ATTACH each schema file under its schema name (created if missing). The
		// path is escaped as a SQL string literal (single quotes doubled).
		for schema in SCHEMAS
		{
			let file = match schema
			{
				"production" => paths.production_database_file(),
				_ => paths.sandbox_database_file(),
			};
			let escaped = file.to_string_lossy().replace('\'', "''");
			connection.execute_batch(&format!("ATTACH DATABASE '{escaped}' AS {schema};"))?;
		}

		let database = Self {
			connection,
			paths: paths.clone(),
			active_schema: active_schema.to_string(),
			backup_min_interval: Duration::from_secs(backup_min_interval_seconds),
			backups_pruned_after_days,
			last_backup_at: None,
		};

		for schema in SCHEMAS
		{
			database.migrate_schema(schema)?;
		}

		Ok(database)
	}

	// Point subsequent queries at a different schema (run-mode switch). Instant —
	// both schemas remain attached.
	pub fn set_active_schema(&mut self, schema: &str)
	{
		self.active_schema = schema.to_string();
	}

	// Apply outstanding migrations to one schema, keyed by its own user_version.
	fn migrate_schema(&self, schema: &str) -> AppResult<()>
	{
		let version: i64 = self
			.connection
			.query_row(&format!("PRAGMA {schema}.user_version"), [], |row| row.get(0))?;

		// v0 -> v1: create the initial schema and seed defaults.
		if version < 1
		{
			self.connection.execute_batch(&schema_v1_ddl(schema))?;

			let now = Utc::now().to_rfc3339();
			for (kind, name) in DEFAULT_SUBCATEGORIES
			{
				self.connection.execute(
					&format!(
						"INSERT INTO {schema}.subcategories (kind, name, description, created_at)
						 VALUES (?1, ?2, '', ?3)"
					),
					params![kind, name, now],
				)?;
			}
			for (code, kind, label) in DEFAULT_HMRC_CATEGORIES
			{
				self.connection.execute(
					&format!(
						"INSERT INTO {schema}.hmrc_categories (code, kind, label, description, updated_at)
						 VALUES (?1, ?2, ?3, '', ?4)"
					),
					params![code, kind, label, now],
				)?;
			}
			self.connection
				.execute_batch(&format!("PRAGMA {schema}.user_version = 1;"))?;
		}

		// v1 -> v2: a key/value table for small app metadata (per schema).
		if version < 2
		{
			self.connection.execute_batch(&format!(
				"CREATE TABLE {schema}.app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
				 PRAGMA {schema}.user_version = 2;"
			))?;
		}

		Ok(())
	}

	// Read a value from the active schema's app_meta table (None if absent).
	pub fn get_meta(&self, key: &str) -> AppResult<Option<String>>
	{
		let s = &self.active_schema;
		let value = self
			.connection
			.query_row(
				&format!("SELECT value FROM {s}.app_meta WHERE key = ?1"),
				params![key],
				|row| row.get::<_, String>(0),
			)
			.optional()?;
		Ok(value)
	}

	// Insert or update a value in the active schema's app_meta table.
	pub fn set_meta(&self, key: &str, value: &str) -> AppResult<()>
	{
		let s = &self.active_schema;
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.app_meta (key, value) VALUES (?1, ?2)
				 ON CONFLICT(key) DO UPDATE SET value = excluded.value"
			),
			params![key, value],
		)?;
		Ok(())
	}

	// Update the backup/retention knobs after the user changes settings.
	pub fn update_retention_settings(
		&mut self,
		backup_min_interval_seconds: u64,
		backups_pruned_after_days: u32,
	)
	{
		self.backup_min_interval = Duration::from_secs(backup_min_interval_seconds);
		self.backups_pruned_after_days = backups_pruned_after_days;
	}

	// Take a backup of the active schema before a mutation, debounced so a batch of
	// writes produces one backup, not hundreds.
	fn maybe_backup(&mut self) -> AppResult<()>
	{
		let is_due = match self.last_backup_at
		{
			None => true,
			Some(previous) => previous.elapsed() >= self.backup_min_interval,
		};
		if !is_due
		{
			return Ok(());
		}

		let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S-%3f").to_string();
		let schema = &self.active_schema;
		let backup_path = self
			.paths
			.backups_directory()
			.join(format!("{stamp}_MyOpenUKTaxApp.{schema}.db"));
		let escaped_path = backup_path.to_string_lossy().replace('\'', "''");

		// `VACUUM <schema> INTO <file>` writes a consistent copy of just that schema.
		self.connection
			.execute_batch(&format!("VACUUM {schema} INTO '{escaped_path}'"))?;

		self.last_backup_at = Some(Instant::now());
		housekeeping::prune_directory_by_age(
			&self.paths.backups_directory(),
			self.backups_pruned_after_days,
		)?;
		Ok(())
	}

	// ---- Subcategories ----------------------------------------------------

	pub fn list_subcategories(&self, kind: Option<&str>) -> AppResult<Vec<Subcategory>>
	{
		let s = &self.active_schema;
		let mut sql = format!(
			"SELECT s.id, s.kind, s.name, s.description, s.created_at,
			        EXISTS (SELECT 1 FROM {s}.ledger_events e WHERE e.subcategory_id = s.id)
			 FROM {s}.subcategories s"
		);
		if kind.is_some()
		{
			sql.push_str(" WHERE s.kind = ?1");
		}
		sql.push_str(" ORDER BY s.kind, s.name");

		let mut statement = self.connection.prepare(&sql)?;
		let map_row = |row: &rusqlite::Row| -> rusqlite::Result<Subcategory> {
			Ok(Subcategory {
				id: row.get(0)?,
				kind: row.get(1)?,
				name: row.get(2)?,
				description: row.get(3)?,
				created_at: row.get(4)?,
				in_use: row.get(5)?,
			})
		};
		let rows = match kind
		{
			Some(kind) => statement.query_map(params![kind], map_row)?,
			None => statement.query_map([], map_row)?,
		};
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	pub fn create_subcategory(
		&mut self,
		kind: &str,
		name: &str,
		description: &str,
	) -> AppResult<Subcategory>
	{
		validate_kind(kind)?;
		let trimmed_name = name.trim();
		if trimmed_name.is_empty()
		{
			return Err(AppError::Validation("subcategory name cannot be empty".to_string()));
		}

		self.maybe_backup()?;
		let s = &self.active_schema;
		let now = Utc::now().to_rfc3339();
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.subcategories (kind, name, description, created_at)
				 VALUES (?1, ?2, ?3, ?4)"
			),
			params![kind, trimmed_name, description.trim(), now],
		)?;

		let new_id = self.connection.last_insert_rowid();
		self.get_subcategory(new_id)
	}

	pub fn get_subcategory(&self, id: i64) -> AppResult<Subcategory>
	{
		let s = &self.active_schema;
		let subcategory = self.connection.query_row(
			&format!(
				"SELECT s.id, s.kind, s.name, s.description, s.created_at,
				        EXISTS (SELECT 1 FROM {s}.ledger_events e WHERE e.subcategory_id = s.id)
				 FROM {s}.subcategories s WHERE s.id = ?1"
			),
			params![id],
			|row| {
				Ok(Subcategory {
					id: row.get(0)?,
					kind: row.get(1)?,
					name: row.get(2)?,
					description: row.get(3)?,
					created_at: row.get(4)?,
					in_use: row.get(5)?,
				})
			},
		);
		subcategory.map_err(|_| AppError::NotFound(format!("subcategory {id}")))
	}

	pub fn update_subcategory(
		&mut self,
		id: i64,
		name: &str,
		description: &str,
	) -> AppResult<Subcategory>
	{
		let trimmed_name = name.trim();
		if trimmed_name.is_empty()
		{
			return Err(AppError::Validation("subcategory name cannot be empty".to_string()));
		}

		self.maybe_backup()?;
		let s = &self.active_schema;
		let affected = self.connection.execute(
			&format!("UPDATE {s}.subcategories SET name = ?2, description = ?3 WHERE id = ?1"),
			params![id, trimmed_name, description.trim()],
		)?;
		if affected == 0
		{
			return Err(AppError::NotFound(format!("subcategory {id}")));
		}
		self.get_subcategory(id)
	}

	pub fn delete_subcategory(&mut self, id: i64) -> AppResult<()>
	{
		let subcategory = self.get_subcategory(id)?;
		if subcategory.in_use
		{
			return Err(AppError::Validation(
				"this category is used by at least one event and can only be renamed".to_string(),
			));
		}

		self.maybe_backup()?;
		let s = &self.active_schema;
		self.connection
			.execute(&format!("DELETE FROM {s}.subcategories WHERE id = ?1"), params![id])?;
		Ok(())
	}

	// ---- Ledger events ----------------------------------------------------

	pub fn list_events(&self, kind: &str, filter: &EventFilter) -> AppResult<Vec<LedgerEvent>>
	{
		validate_kind(kind)?;
		let s = &self.active_schema;
		let mut sql = format!(
			"SELECT e.id, e.kind, e.event_date, e.subcategory_id, s.name,
			        e.amount_pence, e.note, e.created_at
			 FROM {s}.ledger_events e
			 JOIN {s}.subcategories s ON s.id = e.subcategory_id
			 WHERE e.kind = ?1"
		);
		let mut bound: Vec<String> = vec![kind.to_string()];

		if let Some(date_from) = filter.date_from.as_deref().filter(|value| !value.is_empty())
		{
			bound.push(date_from.to_string());
			sql.push_str(&format!(" AND e.event_date >= ?{}", bound.len()));
		}
		if let Some(date_to) = filter.date_to.as_deref().filter(|value| !value.is_empty())
		{
			bound.push(date_to.to_string());
			sql.push_str(&format!(" AND e.event_date <= ?{}", bound.len()));
		}
		if let Some(term) = filter.search_term.as_deref().filter(|value| !value.is_empty())
		{
			bound.push(format!("%{term}%"));
			let index = bound.len();
			sql.push_str(&format!(" AND (s.name LIKE ?{index} OR e.note LIKE ?{index})"));
		}
		sql.push_str(" ORDER BY e.event_date DESC, e.id DESC");

		let mut statement = self.connection.prepare(&sql)?;
		let parameters = rusqlite::params_from_iter(bound.iter());
		let rows = statement.query_map(parameters, |row| {
			Ok(LedgerEvent {
				id: row.get(0)?,
				kind: row.get(1)?,
				event_date: row.get(2)?,
				subcategory_id: row.get(3)?,
				subcategory_name: row.get(4)?,
				amount_pence: row.get(5)?,
				note: row.get(6)?,
				created_at: row.get(7)?,
			})
		})?;
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	pub fn get_event(&self, id: i64) -> AppResult<LedgerEvent>
	{
		let s = &self.active_schema;
		let event = self.connection.query_row(
			&format!(
				"SELECT e.id, e.kind, e.event_date, e.subcategory_id, s.name,
				        e.amount_pence, e.note, e.created_at
				 FROM {s}.ledger_events e
				 JOIN {s}.subcategories s ON s.id = e.subcategory_id
				 WHERE e.id = ?1"
			),
			params![id],
			|row| {
				Ok(LedgerEvent {
					id: row.get(0)?,
					kind: row.get(1)?,
					event_date: row.get(2)?,
					subcategory_id: row.get(3)?,
					subcategory_name: row.get(4)?,
					amount_pence: row.get(5)?,
					note: row.get(6)?,
					created_at: row.get(7)?,
				})
			},
		);
		event.map_err(|_| AppError::NotFound(format!("event {id}")))
	}

	pub fn create_event(&mut self, input: &NewLedgerEvent) -> AppResult<LedgerEvent>
	{
		validate_kind(&input.kind)?;
		validate_date(&input.event_date)?;

		if input.amount_pence < 0 || input.amount_pence > MAX_AMOUNT_PENCE
		{
			return Err(AppError::Validation("amount is out of the allowed range".to_string()));
		}

		let subcategory = self.get_subcategory(input.subcategory_id)?;
		if subcategory.kind != input.kind
		{
			return Err(AppError::Validation(format!(
				"subcategory '{}' is a {} category but the event is {}",
				subcategory.name, subcategory.kind, input.kind
			)));
		}

		self.maybe_backup()?;
		let s = &self.active_schema;
		let now = Utc::now().to_rfc3339();
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.ledger_events
				 (kind, event_date, subcategory_id, amount_pence, note, created_at)
				 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
			),
			params![
				input.kind,
				input.event_date,
				input.subcategory_id,
				input.amount_pence,
				input.note.trim(),
				now
			],
		)?;

		let new_id = self.connection.last_insert_rowid();
		self.get_event(new_id)
	}

	pub fn delete_event(&mut self, id: i64) -> AppResult<()>
	{
		self.maybe_backup()?;
		let s = &self.active_schema;
		let affected = self
			.connection
			.execute(&format!("DELETE FROM {s}.ledger_events WHERE id = ?1"), params![id])?;
		if affected == 0
		{
			return Err(AppError::NotFound(format!("event {id}")));
		}
		Ok(())
	}

	pub fn last_used_subcategory_id(&self, kind: &str) -> AppResult<Option<i64>>
	{
		validate_kind(kind)?;
		let s = &self.active_schema;
		let result = self.connection.query_row(
			&format!(
				"SELECT subcategory_id FROM {s}.ledger_events WHERE kind = ?1 ORDER BY id DESC LIMIT 1"
			),
			params![kind],
			|row| row.get::<_, i64>(0),
		);
		match result
		{
			Ok(id) => Ok(Some(id)),
			Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	// ---- HMRC categories (read-only to the user) --------------------------

	pub fn list_hmrc_categories(&self, kind: Option<&str>) -> AppResult<Vec<HmrcCategory>>
	{
		let s = &self.active_schema;
		let mut sql =
			format!("SELECT id, code, kind, label, description, updated_at FROM {s}.hmrc_categories");
		if kind.is_some()
		{
			sql.push_str(" WHERE kind = ?1");
		}
		sql.push_str(" ORDER BY kind, label");

		let mut statement = self.connection.prepare(&sql)?;
		let map_row = |row: &rusqlite::Row| -> rusqlite::Result<HmrcCategory> {
			Ok(HmrcCategory {
				id: row.get(0)?,
				code: row.get(1)?,
				kind: row.get(2)?,
				label: row.get(3)?,
				description: row.get(4)?,
				updated_at: row.get(5)?,
			})
		};
		let rows = match kind
		{
			Some(kind) => statement.query_map(params![kind], map_row)?,
			None => statement.query_map([], map_row)?,
		};
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	#[allow(dead_code)]
	pub fn upsert_hmrc_category(
		&mut self,
		code: &str,
		kind: &str,
		label: &str,
		description: &str,
	) -> AppResult<()>
	{
		self.maybe_backup()?;
		let s = &self.active_schema;
		let now = Utc::now().to_rfc3339();
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.hmrc_categories (code, kind, label, description, updated_at)
				 VALUES (?1, ?2, ?3, ?4, ?5)
				 ON CONFLICT(code) DO UPDATE SET
				     kind = excluded.kind,
				     label = excluded.label,
				     description = excluded.description,
				     updated_at = excluded.updated_at"
			),
			params![code, kind, label, description, now],
		)?;
		Ok(())
	}

	// ---- Category mappings ------------------------------------------------

	pub fn list_mappings(&self) -> AppResult<Vec<CategoryMapping>>
	{
		let s = &self.active_schema;
		let mut statement = self.connection.prepare(&format!(
			"SELECT m.id, m.subcategory_id, s.kind, s.name,
			        m.hmrc_category_id, h.code, h.label, m.created_at
			 FROM {s}.category_mappings m
			 JOIN {s}.subcategories s ON s.id = m.subcategory_id
			 JOIN {s}.hmrc_categories h ON h.id = m.hmrc_category_id
			 ORDER BY s.kind, s.name"
		))?;
		let rows = statement.query_map([], |row| {
			Ok(CategoryMapping {
				id: row.get(0)?,
				subcategory_id: row.get(1)?,
				subcategory_kind: row.get(2)?,
				subcategory_name: row.get(3)?,
				hmrc_category_id: row.get(4)?,
				hmrc_category_code: row.get(5)?,
				hmrc_category_label: row.get(6)?,
				created_at: row.get(7)?,
			})
		})?;
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	pub fn set_mapping(&mut self, input: &NewCategoryMapping) -> AppResult<()>
	{
		self.maybe_backup()?;
		let s = &self.active_schema;
		let now = Utc::now().to_rfc3339();
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.category_mappings (subcategory_id, hmrc_category_id, created_at)
				 VALUES (?1, ?2, ?3)
				 ON CONFLICT(subcategory_id) DO UPDATE SET
				     hmrc_category_id = excluded.hmrc_category_id,
				     created_at = excluded.created_at"
			),
			params![input.subcategory_id, input.hmrc_category_id, now],
		)?;
		Ok(())
	}

	pub fn delete_mapping(&mut self, id: i64) -> AppResult<()>
	{
		self.maybe_backup()?;
		let s = &self.active_schema;
		self.connection
			.execute(&format!("DELETE FROM {s}.category_mappings WHERE id = ?1"), params![id])?;
		Ok(())
	}

	// ---- HMRC submissions (post history) ----------------------------------

	pub fn list_submissions(&self) -> AppResult<Vec<HmrcSubmission>>
	{
		let s = &self.active_schema;
		let mut statement = self.connection.prepare(&format!(
			"SELECT id, period_from, period_to, submitted_at, status, reference,
			        request_json, response_json
			 FROM {s}.hmrc_submissions ORDER BY submitted_at DESC, id DESC"
		))?;
		let rows = statement.query_map([], |row| {
			Ok(HmrcSubmission {
				id: row.get(0)?,
				period_from: row.get(1)?,
				period_to: row.get(2)?,
				submitted_at: row.get(3)?,
				status: row.get(4)?,
				reference: row.get(5)?,
				request_json: row.get(6)?,
				response_json: row.get(7)?,
			})
		})?;
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	pub fn record_submission(
		&mut self,
		period_from: &str,
		period_to: &str,
		status: &str,
		reference: &str,
		request_json: &str,
		response_json: &str,
	) -> AppResult<HmrcSubmission>
	{
		self.maybe_backup()?;
		let s = &self.active_schema;
		let now = Utc::now().to_rfc3339();
		self.connection.execute(
			&format!(
				"INSERT INTO {s}.hmrc_submissions
				 (period_from, period_to, submitted_at, status, reference, request_json, response_json)
				 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
			),
			params![period_from, period_to, now, status, reference, request_json, response_json],
		)?;
		let new_id = self.connection.last_insert_rowid();

		let submission = self.connection.query_row(
			&format!(
				"SELECT id, period_from, period_to, submitted_at, status, reference,
				        request_json, response_json
				 FROM {s}.hmrc_submissions WHERE id = ?1"
			),
			params![new_id],
			|row| {
				Ok(HmrcSubmission {
					id: row.get(0)?,
					period_from: row.get(1)?,
					period_to: row.get(2)?,
					submitted_at: row.get(3)?,
					status: row.get(4)?,
					reference: row.get(5)?,
					request_json: row.get(6)?,
					response_json: row.get(7)?,
				})
			},
		)?;
		Ok(submission)
	}

	// ---- Dashboard aggregation -------------------------------------------

	pub fn dashboard_summary(
		&self,
		date_from: Option<&str>,
		date_to: Option<&str>,
	) -> AppResult<DashboardSummary>
	{
		let filter = EventFilter {
			date_from: date_from.map(|value| value.to_string()),
			date_to: date_to.map(|value| value.to_string()),
			search_term: None,
		};

		let income_events = self.list_events(KIND_INCOME, &filter)?;
		let expense_events = self.list_events(KIND_EXPENSE, &filter)?;

		let total_income_pence: i64 = income_events.iter().map(|event| event.amount_pence).sum();
		let total_expense_pence: i64 = expense_events.iter().map(|event| event.amount_pence).sum();

		let mut breakdown: Vec<SubcategoryTotal> = Vec::new();
		for event in income_events.iter().chain(expense_events.iter())
		{
			match breakdown
				.iter_mut()
				.find(|entry| entry.subcategory_id == event.subcategory_id)
			{
				Some(entry) =>
				{
					entry.total_pence += event.amount_pence;
					entry.event_count += 1;
				}
				None => breakdown.push(SubcategoryTotal {
					subcategory_id: event.subcategory_id,
					subcategory_name: event.subcategory_name.clone(),
					kind: event.kind.clone(),
					total_pence: event.amount_pence,
					event_count: 1,
				}),
			}
		}
		breakdown.sort_by(|a, b| b.total_pence.cmp(&a.total_pence));

		Ok(DashboardSummary {
			period_from: date_from.unwrap_or("").to_string(),
			period_to: date_to.unwrap_or("").to_string(),
			total_income_pence,
			total_expense_pence,
			net_pence: total_income_pence - total_expense_pence,
			income_event_count: income_events.len() as i64,
			expense_event_count: expense_events.len() as i64,
			breakdown,
		})
	}

	// ---- HMRC submission aggregation -------------------------------------

	pub fn period_totals_by_hmrc_code(
		&self,
		date_from: &str,
		date_to: &str,
	) -> AppResult<Vec<(String, String, i64)>>
	{
		validate_date(date_from)?;
		validate_date(date_to)?;
		let s = &self.active_schema;
		let mut statement = self.connection.prepare(&format!(
			"SELECT h.code, h.kind, SUM(e.amount_pence)
			 FROM {s}.ledger_events e
			 JOIN {s}.category_mappings m ON m.subcategory_id = e.subcategory_id
			 JOIN {s}.hmrc_categories h ON h.id = m.hmrc_category_id
			 WHERE e.event_date >= ?1 AND e.event_date <= ?2
			 GROUP BY h.code, h.kind"
		))?;
		let rows = statement.query_map(params![date_from, date_to], |row| {
			Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
		})?;
		Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
	}

	pub fn unmapped_event_count(&self, date_from: &str, date_to: &str) -> AppResult<i64>
	{
		let s = &self.active_schema;
		let count: i64 = self.connection.query_row(
			&format!(
				"SELECT COUNT(*)
				 FROM {s}.ledger_events e
				 WHERE e.event_date >= ?1 AND e.event_date <= ?2
				   AND NOT EXISTS (
				       SELECT 1 FROM {s}.category_mappings m WHERE m.subcategory_id = e.subcategory_id
				   )"
			),
			params![date_from, date_to],
			|row| row.get(0),
		)?;
		Ok(count)
	}
}

// Validate that a date string is a real calendar date in YYYY-MM-DD form.
fn validate_date(value: &str) -> AppResult<()>
{
	NaiveDate::parse_from_str(value, "%Y-%m-%d")
		.map(|_| ())
		.map_err(|_| AppError::Validation(format!("'{value}' is not a valid YYYY-MM-DD date")))
}
