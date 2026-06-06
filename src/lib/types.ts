// TypeScript mirrors of the Rust backend models.
//
// Field names are snake_case to match exactly what the Tauri commands serialise
// (serde uses the Rust field names), so no case translation is needed at the
// boundary. Money is always an integer number of pence.

export type Kind = "income" | "expense";

export interface Subcategory
{
	id: number;
	kind: Kind;
	name: string;
	description: string;
	created_at: string;
	in_use: boolean;
}

export interface LedgerEvent
{
	id: number;
	kind: Kind;
	event_date: string;
	subcategory_id: number;
	subcategory_name: string;
	amount_pence: number;
	note: string;
	created_at: string;
}

export interface NewLedgerEvent
{
	kind: Kind;
	event_date: string;
	subcategory_id: number;
	amount_pence: number;
	note: string;
}

export interface HmrcCategory
{
	id: number;
	code: string;
	kind: Kind;
	label: string;
	description: string;
	updated_at: string;
}

export interface CategoryMapping
{
	id: number;
	subcategory_id: number;
	subcategory_kind: Kind;
	subcategory_name: string;
	hmrc_category_id: number;
	hmrc_category_code: string;
	hmrc_category_label: string;
	created_at: string;
}

export interface NewCategoryMapping
{
	subcategory_id: number;
	hmrc_category_id: number;
}

export interface HmrcSubmission
{
	id: number;
	period_from: string;
	period_to: string;
	submitted_at: string;
	status: string;
	reference: string;
	request_json: string;
	response_json: string;
}

export interface SubcategoryTotal
{
	subcategory_id: number;
	subcategory_name: string;
	kind: Kind;
	total_pence: number;
	event_count: number;
}

export interface DashboardSummary
{
	period_from: string;
	period_to: string;
	total_income_pence: number;
	total_expense_pence: number;
	net_pence: number;
	income_event_count: number;
	expense_event_count: number;
	breakdown: SubcategoryTotal[];
}

export interface EventFilter
{
	date_from?: string | null;
	date_to?: string | null;
	search_term?: string | null;
}

export interface HmrcSettings
{
	environment: string;
	client_id: string;
	client_secret: string;
	redirect_uri: string;
	national_insurance_number: string;
	business_id: string;
	access_token: string;
	refresh_token: string;
	token_expires_at_epoch_seconds: number;
}

export interface Settings
{
	device_id: string;
	theme: string;
	font_size: string;
	backups_pruned_after_days: number;
	logs_pruned_after_days: number;
	backup_min_interval_seconds: number;
	mcp_server_enabled: boolean;
	mcp_server_port: number;
	hmrc: HmrcSettings;
}

export interface HmrcStatus
{
	configured: boolean;
	business_configured: boolean;
	has_token: boolean;
	environment: string;
	token_expires_at_epoch_seconds: number;
}

export interface HmrcApiResult
{
	status: number;
	body: unknown;
}

// The allowed appearance options, kept in sync with the Rust settings validator.
export const THEME_OPTIONS = ["system", "light", "dark"] as const;
export const FONT_SIZE_OPTIONS = [
	"xxx-small",
	"xx-small",
	"x-small",
	"small",
	"medium",
	"large",
	"x-large",
	"xx-large",
	"xxx-large",
] as const;
export const HMRC_ENVIRONMENT_OPTIONS = ["sandbox", "production"] as const;
