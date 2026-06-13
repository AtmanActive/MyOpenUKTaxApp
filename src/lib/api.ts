// Typed wrappers around the Tauri command surface.
//
// Every backend command is reached through this single module so the rest of
// the UI never touches `invoke` directly. Argument keys are snake_case to match
// the `rename_all = "snake_case"` commands on the Rust side.

import { invoke } from "@tauri-apps/api/core";
import type {
	AppInfo,
	CategoryMapping,
	DashboardSummary,
	EventFilter,
	HmrcApiResult,
	HmrcBusiness,
	HmrcCategory,
	HmrcStatus,
	HmrcSubmission,
	Kind,
	LastUsedSubcategories,
	LedgerEvent,
	NewCategoryMapping,
	NewLedgerEvent,
	RunMode,
	Settings,
	Subcategory,
	TestDataSetup,
	UpdateCheck,
} from "@/lib/types";

export const api = {
	// ---- Subcategories ----
	list_subcategories: (kind?: Kind) =>
		invoke<Subcategory[]>("list_subcategories", { kind: kind ?? null }),
	create_subcategory: (kind: Kind, name: string, description: string) =>
		invoke<Subcategory>("create_subcategory", { kind, name, description }),
	update_subcategory: (id: number, name: string, description: string) =>
		invoke<Subcategory>("update_subcategory", { id, name, description }),
	delete_subcategory: (id: number) => invoke<void>("delete_subcategory", { id }),

	// ---- Ledger events ----
	list_events: (kind: Kind, filter?: EventFilter) =>
		invoke<LedgerEvent[]>("list_events", { kind, filter: filter ?? null }),
	get_event: (id: number) => invoke<LedgerEvent>("get_event", { id }),
	create_event: (input: NewLedgerEvent) =>
		invoke<LedgerEvent>("create_event", { input }),
	delete_event: (id: number) => invoke<void>("delete_event", { id }),
	last_used_subcategories: () =>
		invoke<LastUsedSubcategories>("last_used_subcategories"),

	// ---- Category mappings ----
	list_category_mappings: () =>
		invoke<CategoryMapping[]>("list_category_mappings"),
	set_category_mapping: (input: NewCategoryMapping) =>
		invoke<void>("set_category_mapping", { input }),
	delete_category_mapping: (id: number) =>
		invoke<void>("delete_category_mapping", { id }),

	// ---- Dashboard ----
	get_dashboard_summary: (date_from?: string, date_to?: string) =>
		invoke<DashboardSummary>("get_dashboard_summary", {
			date_from: date_from ?? null,
			date_to: date_to ?? null,
		}),

	// ---- Settings ----
	get_settings: () => invoke<Settings>("get_settings"),
	update_settings: (settings: Settings) =>
		invoke<Settings>("update_settings", { settings }),
	// Restarts the app so startup-only settings (the MCP server) take effect.
	restart_app: () => invoke<void>("restart_app"),
	// Open the portable Data / Logs folders in the OS file explorer.
	open_data_directory: () => invoke<void>("open_data_directory"),
	open_logs_directory: () => invoke<void>("open_logs_directory"),

	// ---- App info & updates ----
	app_info: () => invoke<AppInfo>("app_info"),
	check_latest_version: () => invoke<UpdateCheck>("check_latest_version"),

	// ---- HMRC ----
	list_hmrc_categories: (kind?: Kind) =>
		invoke<HmrcCategory[]>("list_hmrc_categories", { kind: kind ?? null }),
	list_hmrc_submissions: () =>
		invoke<HmrcSubmission[]>("list_hmrc_submissions"),
	hmrc_list_businesses: () => invoke<HmrcBusiness[]>("hmrc_list_businesses"),
	hmrc_set_business_id: (business_id: string) =>
		invoke<void>("hmrc_set_business_id", { business_id }),
	hmrc_status: () => invoke<HmrcStatus>("hmrc_status"),
	hmrc_redirect_uris: () => invoke<string[]>("hmrc_redirect_uris"),
	// Runs the full loopback authorise flow; resolves once tokens are stored.
	hmrc_authorize: () => invoke<HmrcStatus>("hmrc_authorize"),
	hmrc_hello_world: () => invoke<HmrcApiResult>("hmrc_hello_world"),
	hmrc_refresh_token: () => invoke<HmrcStatus>("hmrc_refresh_token"),
	// Cumulative (year-to-date) submission: only the "reporting up to" date is
	// supplied; the backend derives the tax year and the 6 April start date.
	hmrc_submit_period: (period_end: string) =>
		invoke<HmrcSubmission>("hmrc_submit_period", { period_end }),

	// ---- HMRC Get (read-only, live state from HMRC) ----
	// Each returns the raw {status, body} so the UI can show HMRC's response (and
	// HTTP status) directly, including 404s for APIs the app is not subscribed to.
	hmrc_get_business_details: () => invoke<HmrcApiResult>("hmrc_get_business_details"),
	hmrc_get_obligations_quarterly: () => invoke<HmrcApiResult>("hmrc_get_obligations_quarterly"),
	hmrc_get_obligations_final_declaration: (tax_year: string) =>
		invoke<HmrcApiResult>("hmrc_get_obligations_final_declaration", { tax_year }),
	hmrc_get_cumulative: (tax_year: string) =>
		invoke<HmrcApiResult>("hmrc_get_cumulative", { tax_year }),
	hmrc_get_annual: (tax_year: string) => invoke<HmrcApiResult>("hmrc_get_annual", { tax_year }),
	hmrc_get_period_summaries: (tax_year: string) =>
		invoke<HmrcApiResult>("hmrc_get_period_summaries", { tax_year }),

	// Phase 2: optional APIs needing their own Developer Hub subscription (404 until
	// subscribed).
	hmrc_get_biss: (tax_year: string) => invoke<HmrcApiResult>("hmrc_get_biss", { tax_year }),
	hmrc_get_calculations: (tax_year: string) =>
		invoke<HmrcApiResult>("hmrc_get_calculations", { tax_year }),
	hmrc_get_sa_account: () => invoke<HmrcApiResult>("hmrc_get_sa_account"),

	// Sandbox-only: idempotently seed a stateful test business + ITSA status.
	hmrc_setup_test_data: (tax_year: string) =>
		invoke<TestDataSetup>("hmrc_setup_test_data", { tax_year }),
	// When the sandbox test data was last seeded (RFC3339), or null.
	hmrc_test_data_seeded_at: () => invoke<string | null>("hmrc_test_data_seeded_at"),

	// ---- Run mode (Sandbox / Production) ----
	get_run_mode: () => invoke<RunMode>("get_run_mode"),
	set_run_mode: (mode: RunMode) => invoke<RunMode>("set_run_mode", { mode }),
};
