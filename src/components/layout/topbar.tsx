// The always-present top bar. Its contents change with the active section:
// the Events and Dashboard screens get search/date filters, while the rest just
// show the section title. Filter values live in the shared app store so the
// relevant section can read them.
//
// When any filter (search term or date range) is active the bar makes that
// obvious: a flashing filter icon, a clear (✕) control, a dark-red background
// and a " (filtered)" suffix on the title.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import type { RunMode } from "@/lib/types";
import { cn } from "@/lib/utils";
import { SECTIONS, use_app_store } from "@/store/app-store";
import { notify_error } from "@/store/notify";

export function Topbar()
{
	const active_section = use_app_store((state) => state.active_section);
	const set_active_section = use_app_store((state) => state.set_active_section);
	const search_term = use_app_store((state) => state.search_term);
	const set_search_term = use_app_store((state) => state.set_search_term);
	const date_from = use_app_store((state) => state.date_from);
	const date_to = use_app_store((state) => state.date_to);
	const set_date_range = use_app_store((state) => state.set_date_range);
	const clear_filter = use_app_store((state) => state.clear_filter);

	const meta = SECTIONS.find((section) => section.id === active_section);

	// Search is only meaningful on the recorded-events screen.
	const show_search = active_section === "events";
	// A date window is useful on both the events list and the dashboard.
	const show_dates = active_section === "events" || active_section === "dashboard";
	// Any active filter (only relevant on the screens that show filters).
	const filter_active =
		(show_search || show_dates) &&
		(search_term !== "" || date_from !== "" || date_to !== "");

	return (
		<header
			className={cn(
				"flex flex-wrap items-center gap-3 border-b border-border px-4 py-2 transition-colors",
				filter_active ? "bg-red-900 text-white" : "bg-background",
			)}
		>
			<div className="flex items-center gap-2 text-base font-semibold">
				<Icon
					name={meta?.icon ?? "circle"}
					className={cn("text-xl", filter_active ? "text-red-200" : "text-muted-foreground")}
				/>
				<span>{(meta?.label ?? "") + (filter_active ? " (filtered)" : "")}</span>
			</div>

			<div className="flex flex-1 flex-wrap items-center justify-end gap-2">
				<RunModeToggle />

				{show_search ? (
					<div className="flex items-center gap-2">
						{/* Flashing indicator that a filter is currently applied. */}
						{filter_active ? (
							<Icon
								name="filter_alt"
								title="Filter is active"
								className="animate-flash text-xl text-red-200"
							/>
						) : null}

						<div className="relative">
							<Icon
								name="search"
								className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-base text-muted-foreground"
							/>
							<Input
								className="w-48 pl-8 pr-8"
								placeholder="Search events..."
								title="Filter events by category name or note"
								value={search_term}
								onChange={(event) => set_search_term(event.target.value)}
							/>
							{/* Circled-x to reset the whole filter (search + dates). */}
							{filter_active ? (
								<button
									type="button"
									title="Clear filter"
									onClick={() => clear_filter()}
									className="absolute right-1.5 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-destructive"
								>
									<Icon name="cancel" className="text-base" />
								</button>
							) : null}
						</div>
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
						<span className={filter_active ? "text-red-200" : "text-muted-foreground"}>to</span>
						<Input
							type="date"
							className="w-40"
							title="End date filter"
							value={date_to}
							onChange={(event) => set_date_range(date_from, event.target.value)}
						/>
					</>
				) : null}

				{/* Settings has no filters; offer a close affordance back to the Dashboard. */}
				{active_section === "settings" ? (
					<Button
						variant="ghost"
						size="icon"
						title="Close settings and return to the dashboard"
						onClick={() => set_active_section("dashboard")}
					>
						<Icon name="close" className="text-xl" />
					</Button>
				) : null}
			</div>
		</header>
	);
}

// A Sandbox/Production segmented toggle. Switching is live: it calls the backend
// (which flips the DB schema + HMRC credentials), updates the root mode class, and
// invalidates every query so all screens reload from the newly-active schema.
function RunModeToggle()
{
	const run_mode = use_app_store((state) => state.run_mode);
	const set_run_mode = use_app_store((state) => state.set_run_mode);
	const set_hmrc_connection = use_app_store((state) => state.set_hmrc_connection);
	const query_client = useQueryClient();

	const switch_mutation = useMutation({
		mutationFn: (mode: RunMode) => api.set_run_mode(mode),
		onSuccess: (mode) =>
		{
			set_run_mode(mode);
			// The new mode has its own token/credentials; re-seed the LED and reload
			// every screen's data from the now-active schema.
			set_hmrc_connection("unknown");
			void query_client.invalidateQueries();
		},
		onError: (error) => notify_error(error),
	});

	const modes: RunMode[] = ["sandbox", "production"];
	return (
		<div
			className="flex items-center gap-0.5 rounded-md border border-border p-0.5"
			title="Switch between Sandbox and Production. Each mode has its own data and HMRC credentials."
		>
			{modes.map((mode) => (
				<button
					key={mode}
					type="button"
					disabled={switch_mutation.isPending}
					onClick={() => mode !== run_mode && switch_mutation.mutate(mode)}
					className={cn(
						"rounded px-2 py-0.5 text-xs font-medium capitalize transition-colors",
						run_mode === mode
							? mode === "production"
								? "bg-red-700 text-white"
								: "bg-primary text-primary-foreground"
							: "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
					)}
				>
					{mode}
				</button>
			))}
		</div>
	);
}
