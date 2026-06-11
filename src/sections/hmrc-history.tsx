// Section 6 - HMRC post history and connection.
//
// Top: the HMRC connection state and the actions to connect (test connectivity,
// one-click authorise via a local loopback redirect, refresh the token) and to
// submit a quarterly period. Bottom: the history of submissions, expandable to
// inspect the exact request/response JSON.

import { Fragment, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { JsonBlock } from "@/components/ui/json-block";
import { Label } from "@/components/ui/label";
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
import { format_datetime } from "@/lib/format";
import { notify_error, use_notify } from "@/store/notify";

export function HmrcHistorySection()
{
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const [period_end, set_period_end] = useState("");
	const [expanded_id, set_expanded_id] = useState<number | null>(null);

	const submissions_query = useQuery({
		queryKey: ["hmrc_submissions"],
		queryFn: () => api.list_hmrc_submissions(),
	});

	const submit_mutation = useMutation({
		mutationFn: () =>
		{
			if (!period_end)
			{
				return Promise.reject("Choose the date you are reporting up to.");
			}
			return api.hmrc_submit_period(period_end);
		},
		onSuccess: (submission) =>
		{
			push("info", `Submission recorded: ${submission.status}`);
			void query_client.invalidateQueries({ queryKey: ["hmrc_submissions"] });
		},
		onError: (error) => notify_error(error),
	});

	return (
		<div className="flex flex-col gap-4">
			<Card>
				<CardHeader>
					<CardTitle>Submit cumulative period (year-to-date)</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="flex flex-col gap-3 sm:flex-row sm:items-end">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="period_end">Reporting up to</Label>
							<Input
								id="period_end"
								type="date"
								title="The date you are reporting up to (year-to-date). The tax year and its 6 April start are derived from this."
								value={period_end}
								onChange={(event) => set_period_end(event.target.value)}
							/>
						</div>
						<Button title="Build and submit the cumulative figures to HMRC" onClick={() => submit_mutation.mutate()} disabled={submit_mutation.isPending}>
							<Icon name="cloud_upload" /> Submit
						</Button>
					</div>
					<p className="mt-2 text-xs text-muted-foreground">
						{uk_tax_year_label(period_end) ? (
							<>
								Tax year <span className="font-medium text-foreground">{uk_tax_year_label(period_end)}</span> — cumulative from{" "}
								{tax_year_start(period_end)} to {period_end}.{" "}
							</>
						) : null}
						From 2025-26 onwards HMRC uses cumulative submissions: amounts are aggregated year-to-date from your mapped categories. Unmapped events are excluded.
					</p>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Submission history</CardTitle>
				</CardHeader>
				<CardContent>
					{submissions_query.isLoading ? (
						<Spinner label="Loading…" />
					) : (submissions_query.data ?? []).length === 0 ? (
						<p className="text-sm text-muted-foreground">No submissions yet.</p>
					) : (
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Submitted</TableHead>
									<TableHead>Period</TableHead>
									<TableHead>Status</TableHead>
									<TableHead>Reference</TableHead>
									<TableHead className="text-right">Details</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{(submissions_query.data ?? []).map((submission) => (
									<Fragment key={submission.id}>
										<TableRow>
											<TableCell>{format_datetime(submission.submitted_at)}</TableCell>
											<TableCell>
												{submission.period_from} → {submission.period_to}
											</TableCell>
											<TableCell>{submission.status}</TableCell>
											<TableCell className="text-muted-foreground">{submission.reference || "—"}</TableCell>
											<TableCell className="text-right">
												<Button
													variant="ghost"
													size="icon"
													title="Show request/response JSON"
													onClick={() =>
														set_expanded_id(expanded_id === submission.id ? null : submission.id)
													}
												>
													<Icon name={expanded_id === submission.id ? "expand_less" : "expand_more"} className="text-base" />
												</Button>
											</TableCell>
										</TableRow>
										{expanded_id === submission.id ? (
											<TableRow>
												<TableCell colSpan={5}>
													<div className="grid grid-cols-1 gap-3 md:grid-cols-2">
														<JsonBlock title="Request" data={submission.request_json} />
														<JsonBlock title="Response" data={submission.response_json} />
													</div>
												</TableCell>
											</TableRow>
										) : null}
									</Fragment>
								))}
							</TableBody>
						</Table>
					)}
				</CardContent>
			</Card>
		</div>
	);
}

// The UK tax year starts on 6 April. Returns the start year for a YYYY-MM-DD date,
// or null when the date is empty/invalid.
function tax_year_start_year(end_date: string): number | null
{
	if (!end_date)
	{
		return null;
	}
	const date = new Date(`${end_date}T00:00:00`);
	if (Number.isNaN(date.getTime()))
	{
		return null;
	}
	const year = date.getFullYear();
	const month = date.getMonth() + 1;
	const day = date.getDate();
	return month > 4 || (month === 4 && day >= 6) ? year : year - 1;
}

// The HMRC tax-year label ("YYYY-YY") for a reporting-end date, or null.
function uk_tax_year_label(end_date: string): string | null
{
	const start = tax_year_start_year(end_date);
	if (start === null)
	{
		return null;
	}
	return `${start}-${String((start + 1) % 100).padStart(2, "0")}`;
}

// The 6 April start date ("YYYY-04-06") of the tax year for a reporting-end date.
function tax_year_start(end_date: string): string
{
	const start = tax_year_start_year(end_date);
	return start === null ? "" : `${start}-04-06`;
}
