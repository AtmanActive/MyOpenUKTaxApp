// Section 2 - Add Event.
//
// The primary data-entry form. A tab-switch at the top flips between Income and
// Expenses, recolouring the form and swapping the subcategory list. When opened
// from an existing event it renders read-only with a Clone button that unlocks
// the form pre-populated for a brand-new entry. The automatic entry timestamp
// is recorded by the backend and intentionally not shown.

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { api } from "@/lib/api";
import { parse_pounds_to_pence, today_iso } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Kind } from "@/lib/types";
import { use_app_store } from "@/store/app-store";
import { notify_error, use_notify } from "@/store/notify";

// The editable form shape (amount kept as text while typing).
interface EventForm
{
	kind: Kind;
	event_date: string;
	subcategory_id: number;
	amount: string;
	note: string;
}

function blank_form(kind: Kind): EventForm
{
	return { kind, event_date: today_iso(), subcategory_id: 0, amount: "", note: "" };
}

export function AddEventSection()
{
	const selected_event_id = use_app_store((state) => state.selected_event_id);
	const new_event = use_app_store((state) => state.new_event);
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const [form, set_form] = useState<EventForm>(() => blank_form("income"));
	const [is_readonly, set_is_readonly] = useState(false);

	// All subcategories, filtered client-side by the active kind.
	const subcategories_query = useQuery({
		queryKey: ["subcategories"],
		queryFn: () => api.list_subcategories(),
	});
	const kind_subcategories = (subcategories_query.data ?? []).filter(
		(item) => item.kind === form.kind,
	);

	// The event being viewed/cloned, fetched only when an id is selected.
	const event_query = useQuery({
		queryKey: ["event", selected_event_id],
		queryFn: () => api.get_event(selected_event_id as number),
		enabled: selected_event_id !== null,
	});

	// A blank selection resets to a fresh editable form.
	useEffect(() =>
	{
		if (selected_event_id === null)
		{
			set_form(blank_form("income"));
			set_is_readonly(false);
		}
	}, [selected_event_id]);

	// When the selected event arrives, populate the form read-only.
	useEffect(() =>
	{
		if (selected_event_id !== null && event_query.data)
		{
			const event = event_query.data;
			set_form({
				kind: event.kind,
				event_date: event.event_date,
				subcategory_id: event.subcategory_id,
				amount: (event.amount_pence / 100).toFixed(2),
				note: event.note,
			});
			set_is_readonly(true);
		}
	}, [selected_event_id, event_query.data]);

	// Switch Income/Expense tab; clears the subcategory since the list changes.
	const switch_kind = (kind: Kind) =>
	{
		if (is_readonly)
		{
			return;
		}
		set_form((current) => ({ ...current, kind, subcategory_id: 0 }));
	};

	const create_mutation = useMutation({
		mutationFn: () =>
		{
			const amount_pence = parse_pounds_to_pence(form.amount);
			if (amount_pence === null)
			{
				return Promise.reject("Enter a valid amount in pounds (e.g. 123.45).");
			}
			if (!form.subcategory_id)
			{
				return Promise.reject("Choose a category for this event.");
			}
			return api.create_event({
				kind: form.kind,
				event_date: form.event_date,
				subcategory_id: form.subcategory_id,
				amount_pence,
				note: form.note,
			});
		},
		onSuccess: () =>
		{
			push("success", "Event recorded.");
			void query_client.invalidateQueries({ queryKey: ["events"] });
			void query_client.invalidateQueries({ queryKey: ["dashboard"] });
			// Return to a fresh blank form for the next entry.
			new_event();
			set_form(blank_form(form.kind));
		},
		onError: (error) => notify_error(error),
	});

	const is_income = form.kind === "income";

	return (
		<div className="mx-auto w-full max-w-2xl">
			<Card>
				<CardHeader>
					<CardTitle className="flex items-center justify-between">
						<span>{is_readonly ? "View Event" : "Add Event"}</span>
						{is_readonly ? (
							<span className="flex items-center gap-1 text-sm font-normal text-muted-foreground">
								<Icon name="lock" className="text-base" /> read-only
							</span>
						) : null}
					</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					{/* Income / Expenses tab-switch. */}
					<div className="grid grid-cols-2 gap-2">
						<button
							title="Record an income event"
							disabled={is_readonly}
							onClick={() => switch_kind("income")}
							className={cn(
								"flex items-center justify-center gap-2 rounded-md border py-2 text-sm font-medium transition-colors disabled:opacity-60",
								is_income
									? "border-income bg-income text-income-foreground"
									: "border-input hover:bg-accent",
							)}
						>
							<Icon name="trending_up" className="text-lg" /> Income
						</button>
						<button
							title="Record an expense event"
							disabled={is_readonly}
							onClick={() => switch_kind("expense")}
							className={cn(
								"flex items-center justify-center gap-2 rounded-md border py-2 text-sm font-medium transition-colors disabled:opacity-60",
								!is_income
									? "border-expense bg-expense text-expense-foreground"
									: "border-input hover:bg-accent",
							)}
						>
							<Icon name="trending_down" className="text-lg" /> Expenses
						</button>
					</div>

					<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="event_date">Date</Label>
							<Input
								id="event_date"
								type="date"
								title="The date this income/expense occurred"
								disabled={is_readonly}
								value={form.event_date}
								onChange={(event) =>
									set_form((current) => ({ ...current, event_date: event.target.value }))
								}
							/>
						</div>

						<div className="flex flex-col gap-1.5">
							<Label htmlFor="amount">Amount (GBP)</Label>
							<Input
								id="amount"
								type="number"
								min="0"
								step="0.01"
								inputMode="decimal"
								placeholder="0.00"
								title="Amount in pounds"
								disabled={is_readonly}
								value={form.amount}
								onChange={(event) =>
									set_form((current) => ({ ...current, amount: event.target.value }))
								}
							/>
						</div>
					</div>

					<div className="flex flex-col gap-1.5">
						<Label htmlFor="subcategory">Category</Label>
						<Select
							id="subcategory"
							title="The subcategory for this event"
							disabled={is_readonly}
							value={form.subcategory_id || ""}
							onChange={(event) =>
								set_form((current) => ({
									...current,
									subcategory_id: Number(event.target.value),
								}))
							}
						>
							<option value="" disabled>
								Select a category…
							</option>
							{kind_subcategories.map((item) => (
								<option key={item.id} value={item.id}>
									{item.name}
								</option>
							))}
						</Select>
						{kind_subcategories.length === 0 ? (
							<p className="text-xs text-muted-foreground">
								No {form.kind} categories yet — add one on the Categories screen.
							</p>
						) : null}
					</div>

					<div className="flex flex-col gap-1.5">
						<Label htmlFor="note">Note (optional)</Label>
						<Input
							id="note"
							type="text"
							title="An optional note describing this event"
							disabled={is_readonly}
							value={form.note}
							onChange={(event) =>
								set_form((current) => ({ ...current, note: event.target.value }))
							}
						/>
					</div>

					<div className="flex justify-end gap-2">
						{is_readonly ? (
							<>
								<Button
									variant="outline"
									title="Start a blank new event"
									onClick={() => new_event()}
								>
									<Icon name="note_add" /> New
								</Button>
								<Button
									title="Copy this event into a new editable entry"
									onClick={() => set_is_readonly(false)}
								>
									<Icon name="content_copy" /> Clone
								</Button>
							</>
						) : (
							<Button
								title="Save this event"
								disabled={create_mutation.isPending}
								onClick={() => create_mutation.mutate()}
							>
								<Icon name="save" /> Save event
							</Button>
						)}
					</div>
				</CardContent>
			</Card>
		</div>
	);
}
