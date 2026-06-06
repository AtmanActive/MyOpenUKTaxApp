// Typed wrappers around the Tauri command surface.
//
// Every backend command is reached through this single module so the rest of
// the UI never touches `invoke` directly. Argument keys are snake_case to match
// the `rename_all = "snake_case"` commands on the Rust side.

import { invoke } from "@tauri-apps/api/core";
import type {
	CategoryMapping,
	DashboardSummary,
	EventFilter,
	HmrcApiResult,
	HmrcCategory,
	HmrcStatus,
	HmrcSubmission,
	Kind,
	LedgerEvent,
	NewCategoryMapping,
	NewLedgerEvent,
	Settings,
	Subcategory,
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

	// ---- HMRC ----
	list_hmrc_categories: (kind?: Kind) =>
		invoke<HmrcCategory[]>("list_hmrc_categories", { kind: kind ?? null }),
	list_hmrc_submissions: () =>
		invoke<HmrcSubmission[]>("list_hmrc_submissions"),
	hmrc_status: () => invoke<HmrcStatus>("hmrc_status"),
	hmrc_authorize_url: () => invoke<string>("hmrc_authorize_url"),
	hmrc_hello_world: () => invoke<HmrcApiResult>("hmrc_hello_world"),
	hmrc_exchange_code: (code: string) =>
		invoke<HmrcStatus>("hmrc_exchange_code", { code }),
	hmrc_refresh_token: () => invoke<HmrcStatus>("hmrc_refresh_token"),
	hmrc_submit_period: (period_from: string, period_to: string) =>
		invoke<HmrcSubmission>("hmrc_submit_period", { period_from, period_to }),
};
