// Database row models and the input payloads that cross the Tauri boundary.
//
// Money is always represented as an integer number of pence (`amount_pence`)
// rather than a floating-point pound value, so arithmetic and sums are exact.
// The frontend converts to/from a decimal GBP string at the very edge.
//
// `kind` is kept as a validated string ("income" | "expense") in the models so
// it serialises cleanly to the TypeScript layer without a custom enum decoder.

use crate::error::AppError;
use crate::error::AppResult;
use serde::Deserialize;
use serde::Serialize;

// The two built-in top-level categories. Subcategories and ledger events each
// belong to exactly one of these.
pub const KIND_INCOME: &str = "income";
pub const KIND_EXPENSE: &str = "expense";

// Validate that a free-form string is one of the two allowed kinds.
pub fn validate_kind(kind: &str) -> AppResult<()>
{
	if kind == KIND_INCOME || kind == KIND_EXPENSE
	{
		Ok(())
	}
	else
	{
		Err(AppError::Validation(format!(
			"kind must be '{KIND_INCOME}' or '{KIND_EXPENSE}', got '{kind}'"
		)))
	}
}

// A user-defined subcategory (the user calls these simply "categories").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subcategory
{
	pub id: i64,
	pub kind: String,
	pub name: String,
	pub description: String,
	pub created_at: String,
	// Whether the subcategory is referenced by at least one ledger event; if so
	// the UI must forbid deletion (rename only).
	pub in_use: bool,
}

// A single income or expense ledger entry. `subcategory_name` is denormalised
// from a join so tables can render without a second lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEvent
{
	pub id: i64,
	pub kind: String,
	pub event_date: String,
	pub subcategory_id: i64,
	pub subcategory_name: String,
	pub amount_pence: i64,
	pub note: String,
	pub created_at: String,
}

// Payload for creating a ledger event, sent from the Add Event form.
#[derive(Debug, Clone, Deserialize)]
pub struct NewLedgerEvent
{
	pub kind: String,
	pub event_date: String,
	pub subcategory_id: i64,
	pub amount_pence: i64,
	#[serde(default)]
	pub note: String,
}

// An HMRC category, retrieved from the MTD API and read-only for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrcCategory
{
	pub id: i64,
	pub code: String,
	pub kind: String,
	pub label: String,
	pub description: String,
	pub updated_at: String,
}

// A user mapping from one of their subcategories to a single HMRC category.
// Many subcategories may map to the same HMRC category (many-to-one), so the
// subcategory side is unique while the HMRC side is not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMapping
{
	pub id: i64,
	pub subcategory_id: i64,
	pub subcategory_kind: String,
	pub subcategory_name: String,
	pub hmrc_category_id: i64,
	pub hmrc_category_code: String,
	pub hmrc_category_label: String,
	pub created_at: String,
}

// Payload for creating or replacing a mapping.
#[derive(Debug, Clone, Deserialize)]
pub struct NewCategoryMapping
{
	pub subcategory_id: i64,
	pub hmrc_category_id: i64,
}

// A historical record of a quarterly submission to HMRC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrcSubmission
{
	pub id: i64,
	pub period_from: String,
	pub period_to: String,
	pub submitted_at: String,
	pub status: String,
	pub reference: String,
	pub request_json: String,
	pub response_json: String,
}

// A subcategory total used by the dashboard breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcategoryTotal
{
	pub subcategory_id: i64,
	pub subcategory_name: String,
	pub kind: String,
	pub total_pence: i64,
	pub event_count: i64,
}

// Aggregated figures rendered on the dashboard for a given date window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary
{
	pub period_from: String,
	pub period_to: String,
	pub total_income_pence: i64,
	pub total_expense_pence: i64,
	pub net_pence: i64,
	pub income_event_count: i64,
	pub expense_event_count: i64,
	pub breakdown: Vec<SubcategoryTotal>,
}

// Optional date-range / text filter applied to ledger queries.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventFilter
{
	#[serde(default)]
	pub date_from: Option<String>,
	#[serde(default)]
	pub date_to: Option<String>,
	#[serde(default)]
	pub search_term: Option<String>,
}
