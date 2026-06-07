// Global UI state: which section is active, the cross-section search/date
// filters shown in the topbar, and which event (if any) the Add Event screen
// should display in read-only/clone mode. Kept in a small zustand store so the
// sidebar, topbar and sections can share it without prop drilling.

import { create } from "zustand";
import type { LedgerEvent } from "@/lib/types";

export type SectionId =
	| "dashboard"
	| "add-event"
	| "events"
	| "subcategories"
	| "mapping"
	| "hmrc"
	| "settings";

// Static metadata for each sidebar entry. `icon` is a Material Symbols name.
export interface SectionMeta
{
	id: SectionId;
	label: string;
	icon: string;
	hint: string;
}

export const SECTIONS: SectionMeta[] = [
	{ id: "dashboard", label: "Dashboard", icon: "dashboard", hint: "Overview of your income and expenses" },
	{ id: "add-event", label: "Add Event", icon: "add_circle", hint: "Record a new income or expense event" },
	{ id: "events", label: "Events", icon: "receipt_long", hint: "Browse and filter recorded events" },
	{ id: "subcategories", label: "Categories", icon: "category", hint: "Manage your income and expense categories" },
	{ id: "mapping", label: "Mapping", icon: "swap_horiz", hint: "Map your categories to HMRC categories" },
	{ id: "hmrc", label: "HMRC", icon: "cloud_upload", hint: "Submit quarterly data and view post history" },
	{ id: "settings", label: "Settings", icon: "settings", hint: "Application settings and HMRC connection" },
];

interface AppUiState
{
	active_section: SectionId;
	// The event the Add Event screen should open read-only (null = blank form).
	selected_event_id: number | null;
	// Topbar filter values, shared with the Events and Dashboard sections.
	search_term: string;
	date_from: string;
	date_to: string;

	set_active_section: (section: SectionId) => void;
	// Navigate to Add Event showing an existing event (view / clone source).
	open_event: (id: number) => void;
	// Navigate to Add Event with a blank form.
	new_event: () => void;
	set_search_term: (term: string) => void;
	set_date_range: (from: string, to: string) => void;
	// Clear the whole filter (search term + date range).
	clear_filter: () => void;

	// Up to three most-recently created events this session (newest first), shown
	// as feedback under the Add Event form. In-memory only; not persisted.
	recent_events: LedgerEvent[];
	add_recent_event: (event: LedgerEvent) => void;
}

export const use_app_store = create<AppUiState>((set) => ({
	active_section: "dashboard",
	selected_event_id: null,
	search_term: "",
	date_from: "",
	date_to: "",

	set_active_section: (section) => set({ active_section: section }),
	open_event: (id) => set({ active_section: "add-event", selected_event_id: id }),
	new_event: () => set({ active_section: "add-event", selected_event_id: null }),
	set_search_term: (term) => set({ search_term: term }),
	set_date_range: (from, to) => set({ date_from: from, date_to: to }),
	clear_filter: () => set({ search_term: "", date_from: "", date_to: "" }),

	recent_events: [],
	add_recent_event: (event) =>
		set((state) => ({ recent_events: [event, ...state.recent_events].slice(0, 3) })),
}));
