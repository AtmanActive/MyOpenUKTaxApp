// Section 7 - "HMRC Get": live, read-only state retrieved from HMRC.
//
// Where "HMRC Put" submits, this screen *reads* the authoritative record HMRC
// holds, so it remains a source of truth even if the app lost all local data.
// Each card does a live GET and shows both a friendly view and the raw JSON, so
// nothing HMRC returns is hidden. Commands return the raw {status, body}, so a
// card surfaces HMRC's HTTP status directly — including 404s for APIs the
// application is not (yet) subscribed to.

import type { ReactNode } from "react";
import { useState } from "react";
import { useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { JsonBlock } from "@/components/ui/json-block";
import { Select } from "@/components/ui/select";
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
import type { HmrcApiResult } from "@/lib/types";
import { use_app_store } from "@/store/app-store";

export function HmrcGetSection()
{
	const set_active_section = use_app_store((state) => state.set_active_section);
	const query_client = useQueryClient();

	const years = tax_year_options();
	const [tax_year, set_tax_year] = useState(years[0]);

	const status_query = useQuery({ queryKey: ["hmrc_status"], queryFn: () => api.hmrc_status() });
	// Only fire the (token-protected) reads once we know a token is present.
	const ready = !!status_query.data?.has_token;

	const business_details = useQuery({
		queryKey: ["hmrc-get", "business-details"],
		queryFn: () => api.hmrc_get_business_details(),
		enabled: ready,
		retry: false,
	});
	const quarterly = useQuery({
		queryKey: ["hmrc-get", "obligations-quarterly"],
		queryFn: () => api.hmrc_get_obligations_quarterly(),
		enabled: ready,
		retry: false,
	});
	const final_declaration = useQuery({
		queryKey: ["hmrc-get", "obligations-final", tax_year],
		queryFn: () => api.hmrc_get_obligations_final_declaration(tax_year),
		enabled: ready,
		retry: false,
	});
	const cumulative = useQuery({
		queryKey: ["hmrc-get", "cumulative", tax_year],
		queryFn: () => api.hmrc_get_cumulative(tax_year),
		enabled: ready,
		retry: false,
	});
	const annual = useQuery({
		queryKey: ["hmrc-get", "annual", tax_year],
		queryFn: () => api.hmrc_get_annual(tax_year),
		enabled: ready,
		retry: false,
	});
	const periods = useQuery({
		queryKey: ["hmrc-get", "periods", tax_year],
		queryFn: () => api.hmrc_get_period_summaries(tax_year),
		enabled: ready,
		retry: false,
	});
	// Phase 2: optional APIs (each needs its own Developer Hub subscription).
	const biss = useQuery({
		queryKey: ["hmrc-get", "biss", tax_year],
		queryFn: () => api.hmrc_get_biss(tax_year),
		enabled: ready,
		retry: false,
	});
	const calculations = useQuery({
		queryKey: ["hmrc-get", "calculations", tax_year],
		queryFn: () => api.hmrc_get_calculations(tax_year),
		enabled: ready,
		retry: false,
	});
	const sa_account = useQuery({
		queryKey: ["hmrc-get", "sa-account"],
		queryFn: () => api.hmrc_get_sa_account(),
		enabled: ready,
		retry: false,
	});

	const refresh_all = () => void query_client.invalidateQueries({ queryKey: ["hmrc-get"] });

	const status = status_query.data;

	return (
		<div className="flex flex-col gap-4">
			<Card>
				<CardHeader>
					<CardTitle>HMRC record (live)</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-3">
					{status_query.isLoading || !status ? (
						<Spinner label="Checking status…" />
					) : !status.has_token ? (
						<p className="text-sm text-muted-foreground">
							Not authorised yet. Sign in on the{" "}
							<button className="underline" onClick={() => set_active_section("hmrc")}>
								HMRC Put
							</button>{" "}
							screen first; this screen then reads back what HMRC holds.
						</p>
					) : (
						<div className="flex flex-wrap items-end gap-3">
							<div className="flex flex-col gap-1.5">
								<label htmlFor="get_tax_year" className="text-sm font-medium">
									Tax year
								</label>
								<Select
									id="get_tax_year"
									className="w-40"
									value={tax_year}
									onChange={(event) => set_tax_year(event.target.value)}
								>
									{years.map((year) => (
										<option key={year} value={year}>
											{year}
										</option>
									))}
								</Select>
							</div>
							<Button variant="outline" title="Re-read every card from HMRC" onClick={refresh_all}>
								<Icon name="refresh" /> Refresh all
							</Button>
							<p className="text-xs text-muted-foreground">
								Everything below is read live from HMRC — the authoritative record, independent of
								this app's local data.
							</p>
						</div>
					)}
				</CardContent>
			</Card>

			{ready ? (
				<>
					<ResultCard title="Quarterly obligations (income & expenditure)" query={quarterly}>
						{(result) => <ObligationsTable body={result.body} />}
					</ResultCard>

					<ResultCard title={`Final declaration obligations (${tax_year})`} query={final_declaration}>
						{(result) => <ObligationsTable body={result.body} />}
					</ResultCard>

					<ResultCard title={`Cumulative summary on record (${tax_year})`} query={cumulative}>
						{(result) => <CumulativeView body={result.body} />}
					</ResultCard>

					<ResultCard title="Business details" query={business_details} />

					<ResultCard title={`Annual submission (${tax_year})`} query={annual} />

					<ResultCard title={`Period summaries (${tax_year}, legacy ≤2024-25)`} query={periods}>
						{() => (
							<p className="text-xs text-muted-foreground">
								Legacy endpoint — only tax years 2024-25 and earlier are supported here; from
								2025-26 the cumulative summary above is the record.
							</p>
						)}
					</ResultCard>

					<p className="mt-2 text-sm font-medium text-muted-foreground">
						Additional records — each needs its own HMRC Developer Hub subscription (cards show
						HTTP 404 until you enable them).
					</p>

					<ResultCard title={`Business Income Source Summary (${tax_year})`} query={biss} />

					<ResultCard title={`Tax calculations (${tax_year})`} query={calculations} />

					<ResultCard title="Self Assessment account (open balance & transactions)" query={sa_account} />
				</>
			) : null}
		</div>
	);
}

// A card wrapping one HMRC GET: a refresh control, the HTTP status, an optional
// friendly rendering, and the raw JSON (collapsed) so nothing is hidden.
function ResultCard(props: {
	title: string;
	query: UseQueryResult<HmrcApiResult>;
	children?: (result: HmrcApiResult) => ReactNode;
})
{
	const { data, isFetching, error, refetch } = props.query;

	return (
		<Card>
			<CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
				<CardTitle>{props.title}</CardTitle>
				<div className="flex items-center gap-2">
					{data ? <StatusPill status={data.status} /> : null}
					<Button
						variant="outline"
						size="icon"
						title="Refresh this card from HMRC"
						onClick={() => void refetch()}
						disabled={isFetching}
					>
						<Icon name="refresh" className={isFetching ? "animate-spin" : ""} />
					</Button>
				</div>
			</CardHeader>
			<CardContent className="flex flex-col gap-3">
				{isFetching && !data ? <Spinner label="Loading…" /> : null}
				{error ? (
					<p className="text-sm text-destructive">
						{typeof error === "string" ? error : (error as Error).message}
					</p>
				) : null}
				{data ? (
					<>
						{props.children ? props.children(data) : null}
						<details>
							<summary className="cursor-pointer text-xs text-muted-foreground">Raw JSON</summary>
							<div className="mt-2">
								<JsonBlock data={data.body} />
							</div>
						</details>
					</>
				) : null}
			</CardContent>
		</Card>
	);
}

// An HTTP-status pill, colour-coded so non-2xx responses stand out. A 404 is
// called out as "not found / not subscribed", the most common cause here.
function StatusPill({ status }: { status: number })
{
	const ok = status >= 200 && status < 300;
	const not_found = status === 404;
	const className = ok
		? "bg-income text-income-foreground"
		: not_found
			? "bg-amber-500/20 text-amber-700 dark:text-amber-300"
			: "bg-destructive/15 text-destructive";
	const label = ok
		? `HTTP ${status}`
		: not_found
			? `HTTP ${status} — not found / not subscribed`
			: `HTTP ${status}`;
	return (
		<span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${className}`}>
			{label}
		</span>
	);
}

// ---- Friendly renderers (defensive: HMRC's shapes vary, raw JSON always shown) ----

interface ObligationRow
{
	periodStartDate?: string;
	periodEndDate?: string;
	dueDate?: string;
	status?: string;
	receivedDate?: string;
	periodKey?: string;
	businessId?: string;
	typeOfBusiness?: string;
}

// Flatten HMRC's obligations payload into rows, tolerating both the grouped
// (income-and-expenditure) and flat (crystallisation) shapes.
function flatten_obligations(body: unknown): ObligationRow[]
{
	const obligations = (body as { obligations?: unknown })?.obligations;
	if (!Array.isArray(obligations))
	{
		return [];
	}
	const rows: ObligationRow[] = [];
	for (const entry of obligations as Array<Record<string, unknown>>)
	{
		const details = entry.obligationDetails;
		if (Array.isArray(details))
		{
			for (const detail of details as ObligationRow[])
			{
				rows.push({
					...detail,
					businessId: entry.businessId as string | undefined,
					typeOfBusiness: entry.typeOfBusiness as string | undefined,
				});
			}
		}
		else
		{
			rows.push(entry as ObligationRow);
		}
	}
	return rows;
}

function ObligationsTable({ body }: { body: unknown })
{
	const rows = flatten_obligations(body);
	if (rows.length === 0)
	{
		return <p className="text-sm text-muted-foreground">No obligations returned for this selection.</p>;
	}
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Period</TableHead>
					<TableHead>Due</TableHead>
					<TableHead>Status</TableHead>
					<TableHead>Received</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{rows.map((row, index) => (
					<TableRow key={`${row.periodStartDate ?? index}-${row.periodEndDate ?? ""}`}>
						<TableCell>
							{row.periodStartDate ?? "—"} → {row.periodEndDate ?? "—"}
						</TableCell>
						<TableCell>{row.dueDate ?? "—"}</TableCell>
						<TableCell>
							<ObligationStatusBadge status={row.status} />
						</TableCell>
						<TableCell className="text-muted-foreground">{row.receivedDate ?? "—"}</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	);
}

function ObligationStatusBadge({ status }: { status?: string })
{
	const fulfilled = status === "fulfilled" || status === "Fulfilled";
	return (
		<span
			className={
				"inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium " +
				(fulfilled ? "bg-income text-income-foreground" : "bg-amber-500/20 text-amber-700 dark:text-amber-300")
			}
		>
			<Icon name={fulfilled ? "check_circle" : "schedule"} className="text-sm" />
			{status ?? "unknown"}
		</span>
	);
}

// Render the cumulative summary's period dates plus its income/expense figures.
function CumulativeView({ body }: { body: unknown })
{
	const record = body as {
		periodDates?: { periodStartDate?: string; periodEndDate?: string };
		periodIncome?: Record<string, number>;
		periodExpenses?: Record<string, number>;
	};
	const dates = record?.periodDates;
	const income = record?.periodIncome;
	const expenses = record?.periodExpenses;

	if (!dates && !income && !expenses)
	{
		return <p className="text-sm text-muted-foreground">No cumulative figures on record for this year.</p>;
	}

	return (
		<div className="flex flex-col gap-3">
			{dates ? (
				<p className="text-sm">
					Period: <span className="font-medium">{dates.periodStartDate ?? "—"}</span> →{" "}
					<span className="font-medium">{dates.periodEndDate ?? "—"}</span>
				</p>
			) : null}
			<AmountTable title="Income" amounts={income} />
			<AmountTable title="Expenses" amounts={expenses} />
		</div>
	);
}

function AmountTable({ title, amounts }: { title: string; amounts?: Record<string, number> })
{
	const entries = amounts ? Object.entries(amounts) : [];
	if (entries.length === 0)
	{
		return null;
	}
	return (
		<div>
			<p className="mb-1 text-xs font-semibold text-muted-foreground">{title}</p>
			<Table>
				<TableBody>
					{entries.map(([key, value]) => (
						<TableRow key={key}>
							<TableCell className="font-mono text-xs">{key}</TableCell>
							<TableCell className="text-right">
								{typeof value === "number" ? `£${value.toFixed(2)}` : String(value)}
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</div>
	);
}

// Build the last few UK tax years (newest first) as YYYY-YY labels. The tax year
// starts on 6 April.
function tax_year_options(): string[]
{
	const now = new Date();
	const year = now.getFullYear();
	const month = now.getMonth() + 1;
	const day = now.getDate();
	const start = month > 4 || (month === 4 && day >= 6) ? year : year - 1;

	const options: string[] = [];
	for (let value = start; value >= start - 3; value--)
	{
		options.push(`${value}-${String((value + 1) % 100).padStart(2, "0")}`);
	}
	return options;
}
