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
import { use_app_store } from "@/store/app-store";
import { notify_error, use_notify } from "@/store/notify";

export function HmrcHistorySection()
{
	const push = use_notify((state) => state.push);
	const set_active_section = use_app_store((state) => state.set_active_section);
	const query_client = useQueryClient();

	const [period_from, set_period_from] = useState("");
	const [period_to, set_period_to] = useState("");
	const [expanded_id, set_expanded_id] = useState<number | null>(null);

	const status_query = useQuery({ queryKey: ["hmrc_status"], queryFn: () => api.hmrc_status() });
	const submissions_query = useQuery({
		queryKey: ["hmrc_submissions"],
		queryFn: () => api.list_hmrc_submissions(),
	});

	const refresh_status = () => void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });

	const test_mutation = useMutation({
		mutationFn: () => api.hmrc_hello_world(),
		onSuccess: (result) => push("info", `HMRC responded: HTTP ${result.status}`),
		onError: (error) => notify_error(error),
	});

	// After authorising, validate that the signed-in account matches the entered
	// NINO and pre-fill the Business ID. The read-only List Businesses call is the
	// validator: 200 ⇒ authorised; the backend turns 401/403/400 into clear text.
	const connect_check_mutation = useMutation({
		mutationFn: () => api.hmrc_list_businesses(),
		onSuccess: async (list) =>
		{
			if (list.length === 0)
			{
				push("info", "Authorised, but HMRC shows no businesses for this account/NINO.");
				return;
			}
			if (list.length === 1)
			{
				try
				{
					await api.hmrc_set_business_id(list[0].business_id);
					void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });
					void query_client.invalidateQueries({ queryKey: ["settings"] });
					push("success", `Connected — using your business (${list[0].business_id}).`);
				}
				catch (error)
				{
					notify_error(error);
				}
				return;
			}
			push("info", `Connected — found ${list.length} businesses. Pick one on the Settings screen.`);
		},
		onError: (error) => notify_error(error),
	});

	const authorize_mutation = useMutation({
		mutationFn: () => api.hmrc_authorize(),
		onSuccess: () =>
		{
			push("success", "Authorised — access token stored.");
			refresh_status();
			// Validate the NINO ↔ account pairing and pre-fill the Business ID.
			connect_check_mutation.mutate();
		},
		onError: (error) => notify_error(error),
	});

	const refresh_token_mutation = useMutation({
		mutationFn: () => api.hmrc_refresh_token(),
		onSuccess: () =>
		{
			push("success", "Access token refreshed.");
			refresh_status();
		},
		onError: (error) => notify_error(error),
	});

	const submit_mutation = useMutation({
		mutationFn: () =>
		{
			if (!period_from || !period_to)
			{
				return Promise.reject("Choose both period start and end dates.");
			}
			return api.hmrc_submit_period(period_from, period_to);
		},
		onSuccess: (submission) =>
		{
			push("info", `Submission recorded: ${submission.status}`);
			void query_client.invalidateQueries({ queryKey: ["hmrc_submissions"] });
		},
		onError: (error) => notify_error(error),
	});

	const status = status_query.data;

	return (
		<div className="flex flex-col gap-4">
			<Card>
				<CardHeader>
					<CardTitle>HMRC connection</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					{status_query.isLoading || !status ? (
						<Spinner label="Checking status…" />
					) : (
						<>
							<div className="flex flex-wrap gap-2">
								<StatusBadge ok={status.configured} ok_text={`client configured (${status.environment})`} bad_text="client id not set" />
								<StatusBadge ok={status.business_configured} ok_text="NINO & business id set" bad_text="NINO / business id missing" />
								<StatusBadge ok={status.has_token} ok_text="access token present" bad_text="not authorised" />
							</div>

							{!status.configured ? (
								<p className="text-sm text-muted-foreground">
									Set your HMRC credentials on the{" "}
									<button className="underline" onClick={() => set_active_section("settings")}>
										Settings
									</button>{" "}
									screen first.
								</p>
							) : null}

							<div className="flex flex-wrap gap-2">
								<Button variant="outline" title="Check connectivity to HMRC (no sign-in needed)" onClick={() => test_mutation.mutate()} disabled={test_mutation.isPending}>
									<Icon name="wifi_tethering" /> Test connection
								</Button>
								<Button
									title="Sign in to HMRC in your browser; the app captures the result automatically"
									onClick={() => authorize_mutation.mutate()}
									disabled={
										!status.configured ||
										authorize_mutation.isPending ||
										connect_check_mutation.isPending
									}
								>
									<Icon name="login" />
									{authorize_mutation.isPending
										? "Waiting for sign-in…"
										: connect_check_mutation.isPending
											? "Checking access…"
											: "Authorise with HMRC"}
								</Button>
								<Button variant="outline" title="Refresh the access token" onClick={() => refresh_token_mutation.mutate()} disabled={!status.has_token || refresh_token_mutation.isPending}>
									<Icon name="autorenew" /> Refresh token
								</Button>
							</div>

							{authorize_mutation.isPending ? (
								<p className="flex items-center gap-2 text-sm text-muted-foreground">
									<Icon name="open_in_new" className="text-base" />
									Complete the sign-in in your browser — this screen is waiting for HMRC to
									redirect back, then it finishes automatically.
								</p>
							) : null}
						</>
					)}
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Submit a quarterly period</CardTitle>
				</CardHeader>
				<CardContent>
					<div className="flex flex-col gap-3 sm:flex-row sm:items-end">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="period_from">Period start</Label>
							<Input id="period_from" type="date" title="Quarter start date" value={period_from} onChange={(event) => set_period_from(event.target.value)} />
						</div>
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="period_to">Period end</Label>
							<Input id="period_to" type="date" title="Quarter end date" value={period_to} onChange={(event) => set_period_to(event.target.value)} />
						</div>
						<Button title="Build and submit this period to HMRC" onClick={() => submit_mutation.mutate()} disabled={submit_mutation.isPending}>
							<Icon name="cloud_upload" /> Submit period
						</Button>
					</div>
					<p className="mt-2 text-xs text-muted-foreground">
						Amounts are aggregated from your mapped categories over the chosen window. Unmapped events are excluded.
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
														<JsonBlock title="Request" json={submission.request_json} />
														<JsonBlock title="Response" json={submission.response_json} />
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

// A pass/fail pill for one aspect of the HMRC connection state.
function StatusBadge(props: { ok: boolean; ok_text: string; bad_text: string })
{
	return (
		<span
			className={
				"inline-flex items-center gap-1 rounded-full px-3 py-1 text-xs font-medium " +
				(props.ok ? "bg-income text-income-foreground" : "bg-muted text-muted-foreground")
			}
		>
			<Icon name={props.ok ? "check_circle" : "radio_button_unchecked"} className="text-sm" />
			{props.ok ? props.ok_text : props.bad_text}
		</span>
	);
}

// A scrollable monospaced JSON viewer used in the expanded history rows.
function JsonBlock(props: { title: string; json: string })
{
	return (
		<div>
			<p className="mb-1 text-xs font-semibold text-muted-foreground">{props.title}</p>
			<pre className="max-h-64 overflow-auto rounded-md bg-muted p-2 text-xs">
				{props.json || "—"}
			</pre>
		</div>
	);
}
