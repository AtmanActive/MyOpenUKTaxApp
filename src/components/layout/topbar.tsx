// The always-present top bar. Its contents change with the active section:
// the Events and Dashboard screens get search/date filters, while the rest just
// show the section title. Filter values live in the shared app store so the
// relevant section can read them.

import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { SECTIONS, use_app_store } from "@/store/app-store";

export function Topbar()
{
	const active_section = use_app_store((state) => state.active_section);
	const search_term = use_app_store((state) => state.search_term);
	const set_search_term = use_app_store((state) => state.set_search_term);
	const date_from = use_app_store((state) => state.date_from);
	const date_to = use_app_store((state) => state.date_to);
	const set_date_range = use_app_store((state) => state.set_date_range);

	const meta = SECTIONS.find((section) => section.id === active_section);

	// Search is only meaningful on the recorded-events screen.
	const show_search = active_section === "events";
	// A date window is useful on both the events list and the dashboard.
	const show_dates = active_section === "events" || active_section === "dashboard";

	return (
		<header className="flex flex-wrap items-center gap-3 border-b border-border bg-background px-4 py-2">
			<div className="flex items-center gap-2 text-base font-semibold">
				<Icon name={meta?.icon ?? "circle"} className="text-xl text-muted-foreground" />
				<span>{meta?.label ?? ""}</span>
			</div>

			<div className="flex flex-1 flex-wrap items-center justify-end gap-2">
				{show_search ? (
					<div className="relative">
						<Icon
							name="search"
							className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-base text-muted-foreground"
						/>
						<Input
							className="w-48 pl-8"
							placeholder="Search events..."
							title="Filter events by category name or note"
							value={search_term}
							onChange={(event) => set_search_term(event.target.value)}
						/>
					</div>
				) : null}

				{show_dates ? (
					<>
						<Input
							type="date"
							className="w-40"
							title="Start date filter"
							value={date_from}
							onChange={(event) => set_date_range(event.target.value, date_to)}
						/>
						<span className="text-muted-foreground">to</span>
						<Input
							type="date"
							className="w-40"
							title="End date filter"
							value={date_to}
							onChange={(event) => set_date_range(date_from, event.target.value)}
						/>
					</>
				) : null}
			</div>
		</header>
	);
}
