// Section 1 - Dashboard.
//
// The default view: headline income/expense/net figures plus a per-subcategory
// breakdown for the date window chosen in the topbar. Numbers are clickable to
// jump to the related screen, as the overview requires.

import { useQuery } from "@tanstack/react-query";
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
import { use_app_store } from "@/store/app-store";

export function DashboardSection()
{
	const date_from = use_app_store((state) => state.date_from);
	const date_to = use_app_store((state) => state.date_to);
	const set_active_section = use_app_store((state) => state.set_active_section);

	// Re-fetch whenever the topbar date window changes.
	const summary_query = useQuery({
		queryKey: ["dashboard", date_from, date_to],
		queryFn: () => api.get_dashboard_summary(date_from || undefined, date_to || undefined),
	});

	if (summary_query.isLoading)
	{
		return <Spinner label="Loading dashboard..." />;
	}
	if (summary_query.isError || !summary_query.data)
	{
		return <p className="text-destructive">Could not load the dashboard.</p>;
	}

	const summary = summary_query.data;

	return (
		<div className="flex flex-col gap-4">
			<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
				<StatCard
					title="Total Income"
					icon="trending_up"
					value={format_money_from_pence(summary.total_income_pence)}
					detail={`${summary.income_event_count} events`}
					accent="text-income"
					hint="View income events"
					onClick={() => set_active_section("events")}
				/>
				<StatCard
					title="Total Expenses"
					icon="trending_down"
					value={format_money_from_pence(summary.total_expense_pence)}
					detail={`${summary.expense_event_count} events`}
					accent="text-expense"
					hint="View expense events"
					onClick={() => set_active_section("events")}
				/>
				<StatCard
					title="Net"
					icon="account_balance"
					value={format_money_from_pence(summary.net_pence)}
					detail={summary.net_pence >= 0 ? "in profit" : "in loss"}
					accent={summary.net_pence >= 0 ? "text-income" : "text-expense"}
					hint="Income minus expenses"
				/>
				<StatCard
					title="Period"
					icon="calendar_month"
					value={summary.period_from || summary.period_to ? "Filtered" : "All time"}
					detail={
						summary.period_from || summary.period_to
							? `${summary.period_from || "…"} → ${summary.period_to || "…"}`
							: "Set dates in the toolbar"
					}
					accent="text-primary"
				/>
			</div>

			<Card>
				<CardHeader>
					<CardTitle>Breakdown by category</CardTitle>
				</CardHeader>
				<CardContent>
					{summary.breakdown.length === 0 ? (
						<p className="text-sm text-muted-foreground">
							No events recorded yet. Use “Add Event” to get started.
						</p>
					) : (
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Category</TableHead>
									<TableHead>Type</TableHead>
									<TableHead className="text-right">Events</TableHead>
									<TableHead className="text-right">Total</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{summary.breakdown.map((row) => (
									<TableRow key={`${row.kind}-${row.subcategory_id}`}>
										<TableCell className="font-medium">{row.subcategory_name}</TableCell>
										<TableCell>
											<span
												className={
													row.kind === "income" ? "text-income" : "text-expense"
												}
											>
												{row.kind}
											</span>
										</TableCell>
										<TableCell className="text-right">{row.event_count}</TableCell>
										<TableCell className="text-right tabular-nums">
											{format_money_from_pence(row.total_pence)}
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					)}
				</CardContent>
			</Card>
		</div>
	);
}

// A single clickable headline statistic card.
function StatCard(props: {
	title: string;
	icon: string;
	value: string;
	detail: string;
	accent: string;
	hint?: string;
	onClick?: () => void;
})
{
	return (
		<Card
			title={props.hint}
			onClick={props.onClick}
			className={props.onClick ? "cursor-pointer transition-colors hover:bg-accent" : ""}
		>
			<CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
				<CardTitle className="text-sm text-muted-foreground">{props.title}</CardTitle>
				<Icon name={props.icon} className={`text-xl ${props.accent}`} />
			</CardHeader>
			<CardContent>
				<div className={`text-2xl font-bold tabular-nums ${props.accent}`}>{props.value}</div>
				<p className="text-xs text-muted-foreground">{props.detail}</p>
			</CardContent>
		</Card>
	);
}
