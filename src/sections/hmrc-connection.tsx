// HMRC Connection screen.
//
// The single place for everything about *connecting* to HMRC: credentials, the
// sign-in / token actions, and (in the sandbox) seeding stateful test data. The
// HMRC Put screen submits; HMRC Get reads; this screen gets you connected.
//
// Credential fields auto-save (debounced text, immediate selects/checkbox), the
// same as the Settings screen. The connection actions drive the sidebar LED via
// the shared store (grey → cyan while signing in → green / red).

import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { api } from "@/lib/api";
import { format_datetime } from "@/lib/format";
import type { HmrcBusiness, Settings } from "@/lib/types";
import { use_app_store } from "@/store/app-store";
import { notify_error, use_notify } from "@/store/notify";

type SaveStatus = "idle" | "saving" | "saved";

const DEBOUNCE_MS = 500;

// Normalise a NINO as typed: upper-case, drop spaces, cap at 9 characters.
function normalize_nino(value: string): string
{
	return value.toUpperCase().replace(/\s+/g, "").slice(0, 9);
}

function is_valid_nino(value: string): boolean
{
	return /^[A-Z]{2}[0-9]{6}[A-D]$/.test(value);
}

// HMRC keeps sandbox test data for 7 days; the expiry as an ISO string.
function expiry_of(seeded_at_iso: string): string
{
	const seeded = new Date(seeded_at_iso);
	if (Number.isNaN(seeded.getTime()))
	{
		return seeded_at_iso;
	}
	return new Date(seeded.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString();
}

// Current UK tax year ("YYYY-YY"); the tax year starts on 6 April.
function current_tax_year(): string
{
	const now = new Date();
	const year = now.getFullYear();
	const month = now.getMonth() + 1;
	const day = now.getDate();
	const start = month > 4 || (month === 4 && day >= 6) ? year : year - 1;
	return `${start}-${String((start + 1) % 100).padStart(2, "0")}`;
}

export function HmrcConnectionSection()
{
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();
	const set_hmrc_connection = use_app_store((state) => state.set_hmrc_connection);
	const run_mode = use_app_store((state) => state.run_mode);

	// Credentials are per-mode: the screen edits the active mode's HMRC block.
	const mode_key: "hmrc_sandbox" | "hmrc_production" =
		run_mode === "production" ? "hmrc_production" : "hmrc_sandbox";

	const settings_query = useQuery({ queryKey: ["settings"], queryFn: () => api.get_settings() });
	const status_query = useQuery({ queryKey: ["hmrc_status"], queryFn: () => api.hmrc_status() });
	const redirect_uris_query = useQuery({
		queryKey: ["hmrc_redirect_uris"],
		queryFn: () => api.hmrc_redirect_uris(),
	});

	const [draft, set_draft] = useState<Settings | null>(null);
	const [save_status, set_save_status] = useState<SaveStatus>("idle");
	const draft_ref = useRef<Settings | null>(null);
	const seeded = useRef(false);
	const save_timer = useRef<number | null>(null);

	useEffect(() =>
	{
		if (settings_query.data && !seeded.current)
		{
			seeded.current = true;
			draft_ref.current = settings_query.data;
			set_draft(settings_query.data);
		}
	}, [settings_query.data]);

	useEffect(() => () =>
	{
		if (save_timer.current !== null)
		{
			window.clearTimeout(save_timer.current);
		}
	}, []);

	const persist = (next: Settings, immediate: boolean) =>
	{
		if (save_timer.current !== null)
		{
			window.clearTimeout(save_timer.current);
			save_timer.current = null;
		}
		const run = () =>
		{
			set_save_status("saving");
			api
				.update_settings(next)
				.then((saved) =>
				{
					query_client.setQueryData(["settings"], saved);
					void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });
					set_save_status("saved");
				})
				.catch((error) =>
				{
					notify_error(error);
					set_save_status("idle");
				});
		};
		if (immediate)
		{
			run();
		}
		else
		{
			save_timer.current = window.setTimeout(run, DEBOUNCE_MS);
		}
	};

	const update_hmrc = (patch: Partial<Settings["hmrc_sandbox"]>, immediate: boolean) =>
	{
		const current = draft_ref.current as Settings;
		const next = { ...current, [mode_key]: { ...current[mode_key], ...patch } };
		draft_ref.current = next;
		set_draft(next);
		persist(next, immediate);
	};

	const refresh_status = () => void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });

	// HMRC businesses fetched on demand to populate the Business ID picker.
	const [businesses, set_businesses] = useState<HmrcBusiness[] | null>(null);
	const fetch_businesses_mutation = useMutation({
		mutationFn: () => api.hmrc_list_businesses(),
		onSuccess: (list) =>
		{
			set_businesses(list);
			if (list.length === 1)
			{
				update_hmrc({ business_id: list[0].business_id }, true);
				push("success", "Found one business — selected it.");
			}
			else if (list.length === 0)
			{
				push("info", "No businesses found on your HMRC record.");
			}
		},
		onError: (error) => notify_error(error),
	});

	const business_label = (business: HmrcBusiness): string =>
	{
		const name = business.trading_name || business.type_of_business || "business";
		return `${name} — ${business.business_id}`;
	};

	const test_mutation = useMutation({
		mutationFn: () => api.hmrc_hello_world(),
		onMutate: () => set_hmrc_connection("connecting"),
		onSuccess: (result) =>
		{
			set_hmrc_connection("connected");
			push("info", `HMRC responded: HTTP ${result.status}`);
		},
		onError: (error) =>
		{
			set_hmrc_connection("failed");
			notify_error(error);
		},
	});

	// After authorising, validate the NINO ↔ account pairing and pre-fill the
	// Business ID. The read-only List Businesses call is the validator.
	const connect_check_mutation = useMutation({
		mutationFn: () => api.hmrc_list_businesses(),
		onSuccess: async (list) =>
		{
			set_hmrc_connection("connected");
			if (list.length === 0)
			{
				push("info", "Authorised, but HMRC shows no businesses for this account/NINO.");
				return;
			}
			if (list.length === 1)
			{
				update_hmrc({ business_id: list[0].business_id }, true);
				push("success", `Connected — using your business (${list[0].business_id}).`);
				return;
			}
			set_businesses(list);
			push("info", `Connected — found ${list.length} businesses. Pick one below.`);
		},
		onError: (error) =>
		{
			set_hmrc_connection("failed");
			notify_error(error);
		},
	});

	const authorize_mutation = useMutation({
		mutationFn: () => api.hmrc_authorize(),
		onMutate: () => set_hmrc_connection("connecting"),
		onSuccess: () =>
		{
			push("success", "Authorised — access token stored.");
			refresh_status();
			connect_check_mutation.mutate();
		},
		onError: (error) =>
		{
			set_hmrc_connection("failed");
			notify_error(error);
		},
	});

	const refresh_token_mutation = useMutation({
		mutationFn: () => api.hmrc_refresh_token(),
		onMutate: () => set_hmrc_connection("connecting"),
		onSuccess: () =>
		{
			set_hmrc_connection("connected");
			push("success", "Access token refreshed.");
			refresh_status();
		},
		onError: (error) =>
		{
			set_hmrc_connection("failed");
			notify_error(error);
		},
	});

	const seeded_at_query = useQuery({
		queryKey: ["hmrc_test_data_seeded_at"],
		queryFn: () => api.hmrc_test_data_seeded_at(),
	});

	const setup_mutation = useMutation({
		mutationFn: () => api.hmrc_setup_test_data(current_tax_year()),
		onSuccess: (result) =>
		{
			update_hmrc({ business_id: result.business_id }, true);
			refresh_status();
			void query_client.invalidateQueries({ queryKey: ["hmrc_test_data_seeded_at"] });
			push("success", result.message);
		},
		onError: (error) => notify_error(error),
	});

	if (settings_query.isLoading || !draft)
	{
		return <Spinner label="Loading…" />;
	}

	const status = status_query.data;
	// The active mode's HMRC settings block, read by the credential fields below.
	const hmrc = draft[mode_key];

	return (
		<div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
			<SaveIndicator status={save_status} />

			{/* Production-only warning, shown via the runmode_production class. */}
			<div className="runmode_production flex flex-col gap-1 rounded-md border border-red-700 bg-red-950/40 p-3 text-red-200">
				<span className="flex items-center gap-2 text-sm font-medium">
					<Icon name="warning" className="text-base" /> Production mode
				</span>
				<span className="text-xs">
					You are connected to live HMRC. Credentials and submissions here affect your real
					tax record.
				</span>
			</div>

			{/* Credentials. */}
			<Card>
				<CardHeader>
					<CardTitle>HMRC credentials</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<div className="flex flex-col gap-1.5">
							<Label>Environment</Label>
							<div className="flex h-9 items-center rounded-md border border-input bg-muted px-3 text-sm capitalize text-muted-foreground">
								{run_mode} — switch in the top bar
							</div>
						</div>
						<TextField id="client_id" label="Client ID" value={hmrc.client_id} on_change={(value) => update_hmrc({ client_id: value }, false)} hint="From your HMRC Developer Hub application" />
						<TextField id="client_secret" label="Client secret" type="password" value={hmrc.client_secret} on_change={(value) => update_hmrc({ client_secret: value }, false)} hint="Stored locally in the settings file" />
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="national_insurance_number">National Insurance no.</Label>
							<Input
								id="national_insurance_number"
								title="Your National Insurance number (e.g. AB123456C)"
								placeholder="AB123456C"
								value={hmrc.national_insurance_number}
								onChange={(event) =>
									update_hmrc({ national_insurance_number: normalize_nino(event.target.value) }, false)
								}
							/>
							{hmrc.national_insurance_number.length > 0 &&
							!is_valid_nino(hmrc.national_insurance_number) ? (
								<span className="text-xs text-destructive">
									Doesn’t look like a NINO yet (expected like AB123456C).
								</span>
							) : null}
						</div>
						<TextField id="business_id" label="Business ID" value={hmrc.business_id} on_change={(value) => update_hmrc({ business_id: value }, false)} hint="HMRC business ID (e.g. XAIS…), not your UTR. Use Fetch below to look it up." />
					</div>

					{/* Sandbox-only: hidden in production via the runmode_sandbox class. */}
					<div className="runmode_sandbox flex flex-col gap-4">
						<label
							className="flex items-center gap-2 text-sm"
							title="Send the Gov-Test-Scenario header so HMRC returns its stubbed/stateful test data. Turn off to test against a real identity in the sandbox."
						>
							<input
								type="checkbox"
								className="h-4 w-4"
								checked={hmrc.using_mock_identity}
								onChange={(event) => update_hmrc({ using_mock_identity: event.target.checked }, true)}
							/>
							Using mock identity
						</label>

						{hmrc.using_mock_identity ? (
							<div className="flex flex-col gap-1.5">
								<Label htmlFor="gov_test_scenario">Sandbox test scenario</Label>
								<Input
									id="gov_test_scenario"
									title="Sent as the Gov-Test-Scenario header on every sandbox API call"
									placeholder="e.g. STATEFUL"
									value={hmrc.gov_test_scenario}
									onChange={(event) => update_hmrc({ gov_test_scenario: event.target.value }, false)}
								/>
								<span className="text-xs text-muted-foreground">
									Sent as the Gov-Test-Scenario header on every sandbox call. Use
									<span className="font-mono"> STATEFUL </span>
									so submissions read back and obligations generate.
								</span>
							</div>
						) : null}
					</div>

					<div className="rounded-md border border-border bg-muted/40 p-3">
						<p className="mb-1 text-sm font-medium">
							Register these redirect URIs with your HMRC application
						</p>
						{redirect_uris_query.data && redirect_uris_query.data.length > 0 ? (
							<ul className="list-disc pl-5 font-mono text-xs text-muted-foreground">
								{redirect_uris_query.data.map((uri) => (
									<li key={uri}>{uri}</li>
								))}
							</ul>
						) : (
							<span className="text-xs text-muted-foreground">…</span>
						)}
						<p className="mt-1 text-xs text-muted-foreground">
							Authorise uses whichever of these ports is free at the time, so register all of them.
						</p>
					</div>

					<div className="flex flex-col gap-2">
						<div className="flex flex-wrap items-center gap-2">
							<Button
								variant="outline"
								title="Fetch the businesses on your HMRC record (requires authorisation)"
								disabled={fetch_businesses_mutation.isPending}
								onClick={() => fetch_businesses_mutation.mutate()}
							>
								<Icon name="cloud_download" /> Fetch my businesses
							</Button>
							{fetch_businesses_mutation.isPending ? <Spinner label="Fetching…" /> : null}
						</div>

						{businesses && businesses.length > 1 ? (
							<div className="flex flex-col gap-1.5">
								<Label htmlFor="business_picker">Select your business</Label>
								<Select
									id="business_picker"
									title="Choose which business to use as the Business ID"
									value={hmrc.business_id}
									onChange={(event) => update_hmrc({ business_id: event.target.value }, true)}
								>
									<option value="" disabled>
										Choose…
									</option>
									{businesses.map((business) => (
										<option key={business.business_id} value={business.business_id}>
											{business_label(business)}
										</option>
									))}
								</Select>
							</div>
						) : null}
					</div>
				</CardContent>
			</Card>

			{/* Connection actions. */}
			<Card>
				<CardHeader>
					<CardTitle>Connection</CardTitle>
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
									Fill in your Client ID (and secret) above first.
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

			{/* Sandbox-only stateful test-data seeding (hidden in production via CSS). */}
			<Card className="runmode_sandbox">
					<CardHeader>
						<CardTitle>Sandbox test data</CardTitle>
					</CardHeader>
					<CardContent className="flex flex-col gap-3">
						<p className="text-sm text-muted-foreground">
							Provisions a stateful test business and ITSA status for the test user, so your
							submissions read back and obligations appear (using the STATEFUL scenario, kept by
							HMRC for 7 days). Idempotent — running it again reuses the existing business.
							Needs the <span className="font-medium">MTD SA Test Support API</span> subscription on
							the Developer Hub.
						</p>
						<div className="flex flex-wrap items-center gap-2">
							<Button
								title="Create/verify a stateful test business and ITSA status for the current tax year"
								disabled={setup_mutation.isPending || !status?.has_token}
								onClick={() => setup_mutation.mutate()}
							>
								<Icon name="science" /> Set up test data
							</Button>
							{setup_mutation.isPending ? <Spinner label="Setting up…" /> : null}
							{!status?.has_token ? (
								<span className="text-xs text-muted-foreground">Authorise first.</span>
							) : null}
						</div>
						{setup_mutation.data ? (
							<p className="text-sm text-income">{setup_mutation.data.message}</p>
						) : null}

						{seeded_at_query.data ? (
							<div className="rounded-md border border-border bg-muted/40 p-3 text-sm">
								<p>
									Test data last set up:{" "}
									<span className="font-medium">{format_datetime(seeded_at_query.data)}</span>
								</p>
								<p>
									Expires around:{" "}
									<span className="font-medium">{format_datetime(expiry_of(seeded_at_query.data))}</span>
								</p>
								<p className="mt-1 text-xs text-muted-foreground">
									HMRC deletes sandbox test data 7 days after it is created. After it expires,
									your submissions and obligations will disappear from HMRC Get — click “Set up
									test data” again to re-seed it.
								</p>
							</div>
						) : null}
					</CardContent>
				</Card>
		</div>
	);
}

function SaveIndicator({ status }: { status: SaveStatus })
{
	return (
		<div className="flex h-5 items-center justify-end gap-2 text-sm text-muted-foreground">
			{status === "saving" ? (
				<>
					<Icon name="progress_activity" className="animate-spin text-base" /> Saving…
				</>
			) : null}
			{status === "saved" ? (
				<>
					<Icon name="cloud_done" className="text-base text-income" /> All changes saved
				</>
			) : null}
		</div>
	);
}

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

function TextField(props: {
	id: string;
	label: string;
	value: string;
	on_change: (value: string) => void;
	hint?: string;
	type?: string;
})
{
	return (
		<div className="flex flex-col gap-1.5">
			<Label htmlFor={props.id}>{props.label}</Label>
			<Input
				id={props.id}
				type={props.type ?? "text"}
				title={props.hint}
				value={props.value}
				onChange={(event) => props.on_change(event.target.value)}
			/>
		</div>
	);
}
