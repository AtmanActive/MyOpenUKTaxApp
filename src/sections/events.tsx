// Section 3 - Recorded events.
//
// Two stacked tables (Income on top, Expenses below). Each is sortable by
// clicking a column header; there is no pagination. The search term and date
// window come from the topbar (shared store). Clicking a row opens it on the
// Add Event screen in read-only/clone mode.

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Spinner } from "@/components/ui/spinner";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { format_money_from_pence } from "@/lib/format";
import type { Kind, LedgerEvent } from "@/lib/types";
import { use_app_store } from "@/store/app-store";
import { notify_error, use_notify } from "@/store/notify";

// Sortable columns mapped to the LedgerEvent fields used for comparison.
type SortKey = "event_date" | "subcategory_name" | "amount_pence" | "note";

export function EventsSection()
{
	const search_term = use_app_store((state) => state.search_term);
	const date_from = use_app_store((state) => state.date_from);
	const date_to = use_app_store((state) => state.date_to);

	return (
		<div className="flex flex-col gap-4">
			<EventsTable kind="income" search_term={search_term} date_from={date_from} date_to={date_to} />
			<EventsTable kind="expense" search_term={search_term} date_from={date_from} date_to={date_to} />
		</div>
	);
}

function EventsTable(props: {
	kind: Kind;
	search_term: string;
	date_from: string;
	date_to: string;
})
{
	const open_event = use_app_store((state) => state.open_event);
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const [sort_key, set_sort_key] = useState<SortKey>("event_date");
	const [sort_ascending, set_sort_ascending] = useState(false);

	const events_query = useQuery({
		queryKey: ["events", props.kind, props.date_from, props.date_to, props.search_term],
		queryFn: () =>
			api.list_events(props.kind, {
				date_from: props.date_from || null,
				date_to: props.date_to || null,
				search_term: props.search_term || null,
			}),
	});

	const delete_mutation = useMutation({
		mutationFn: (id: number) => api.delete_event(id),
		onSuccess: () =>
		{
			push("success", "Event deleted.");
			void query_client.invalidateQueries({ queryKey: ["events"] });
			void query_client.invalidateQueries({ queryKey: ["dashboard"] });
		},
		onError: (error) => notify_error(error),
	});

	// Apply the chosen sort to a copy of the rows.
	const sorted_events = useMemo(() =>
	{
		const rows = [...(events_query.data ?? [])];
		rows.sort((left, right) =>
		{
			const comparison = compare_events(left, right, sort_key);
			return sort_ascending ? comparison : -comparison;
		});
		return rows;
	}, [events_query.data, sort_key, sort_ascending]);

	const total_pence = sorted_events.reduce((sum, event) => sum + event.amount_pence, 0);
	const is_income = props.kind === "income";

	// Toggle direction when re-clicking the active column, else switch column.
	const sort_by = (key: SortKey) =>
	{
		if (key === sort_key)
		{
			set_sort_ascending((value) => !value);
		}
		else
		{
			set_sort_key(key);
			set_sort_ascending(false);
		}
	};

	return (
		<Card>
			<CardHeader className="flex-row items-center justify-between space-y-0">
				<CardTitle className={is_income ? "text-income" : "text-expense"}>
					{is_income ? "Income" : "Expenses"}
				</CardTitle>
				<span className="text-sm text-muted-foreground">
					{sorted_events.length} events · {format_money_from_pence(total_pence)}
				</span>
			</CardHeader>
			<CardContent>
				{events_query.isLoading ? (
					<Spinner label="Loading…" />
				) : sorted_events.length === 0 ? (
					<p className="text-sm text-muted-foreground">No matching events.</p>
				) : (
					<Table>
						<TableHeader>
							<TableRow>
								<SortableHeader label="Date" column="event_date" active={sort_key} ascending={sort_ascending} on_click={sort_by} />
								<SortableHeader label="Category" column="subcategory_name" active={sort_key} ascending={sort_ascending} on_click={sort_by} />
								<SortableHeader label="Note" column="note" active={sort_key} ascending={sort_ascending} on_click={sort_by} />
								<SortableHeader label="Amount" column="amount_pence" active={sort_key} ascending={sort_ascending} on_click={sort_by} className="text-right" />
								<TableHead className="text-right">Actions</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody>
							{sorted_events.map((event) => (
								<TableRow key={event.id}>
									<TableCell>{event.event_date}</TableCell>
									<TableCell className="font-medium">{event.subcategory_name}</TableCell>
									<TableCell className="max-w-[16rem] truncate text-muted-foreground">{event.note}</TableCell>
									<TableCell className="text-right tabular-nums">
										{format_money_from_pence(event.amount_pence)}
									</TableCell>
									<TableCell className="text-right">
										<div className="flex justify-end gap-1">
											<Button
												variant="ghost"
												size="icon"
												title="View / clone this event"
												onClick={() => open_event(event.id)}
											>
												<Icon name="visibility" className="text-base" />
											</Button>
											<Button
												variant="ghost"
												size="icon"
												title="Delete this event"
												onClick={() =>
												{
													if (window.confirm("Delete this event?"))
													{
														delete_mutation.mutate(event.id);
													}
												}}
											>
												<Icon name="delete" className="text-base text-destructive" />
											</Button>
										</div>
									</TableCell>
								</TableRow>
							))}
						</TableBody>
					</Table>
				)}
			</CardContent>
		</Card>
	);
}

// A clickable header cell that shows the active sort direction.
function SortableHeader(props: {
	label: string;
	column: SortKey;
	active: SortKey;
	ascending: boolean;
	on_click: (key: SortKey) => void;
	className?: string;
})
{
	const is_active = props.active === props.column;
	return (
		<TableHead className={props.className}>
			<button
				className="inline-flex items-center gap-1 font-medium hover:text-foreground"
				title={`Sort by ${props.label.toLowerCase()}`}
				onClick={() => props.on_click(props.column)}
			>
				{props.label}
				{is_active ? (
					<Icon name={props.ascending ? "arrow_upward" : "arrow_downward"} className="text-sm" />
				) : (
					<Icon name="unfold_more" className="text-sm opacity-40" />
				)}
			</button>
		</TableHead>
	);
}

// Compare two events on the chosen key, returning the usual -1/0/1.
function compare_events(left: LedgerEvent, right: LedgerEvent, key: SortKey): number
{
	if (key === "amount_pence")
	{
		return left.amount_pence - right.amount_pence;
	}
	return String(left[key]).localeCompare(String(right[key]));
}
