// Section 7 - Settings.
//
// Every control auto-saves on change — there is no Save button. Changes apply
// live wherever possible: appearance (theme/font) re-applies because the app
// root re-reads the updated settings cache; HMRC fields are read on each call;
// backup/log retention is pushed straight to the backend. The embedded MCP
// server is the only setting read solely at startup, so when its enable/port
// differ from the values the app launched with, a "Restart now" button appears
// next to it.
//
// Free-text and numeric inputs are debounced; selects and the checkbox save
// immediately so their effect feels instant.

import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { api } from "@/lib/api";
import {
	FONT_SIZE_OPTIONS,
	HMRC_ENVIRONMENT_OPTIONS,
	THEME_OPTIONS,
	type HmrcBusiness,
	type Settings,
	type UpdateCheck,
} from "@/lib/types";
import { notify_error, use_notify } from "@/store/notify";

type SaveStatus = "idle" | "saving" | "saved";

// How long to wait after the last keystroke before persisting a text/number field.
const DEBOUNCE_MS = 500;

// Normalise a NINO as typed: upper-case, drop spaces, cap at 9 characters.
function normalize_nino(value: string): string
{
	return value.toUpperCase().replace(/\s+/g, "").slice(0, 9);
}

// Light shape check mirroring the backend (e.g. AB123456C).
function is_valid_nino(value: string): boolean
{
	return /^[A-Z]{2}[0-9]{6}[A-D]$/.test(value);
}

export function SettingsSection()
{
	const query_client = useQueryClient();
	const push = use_notify((state) => state.push);
	const settings_query = useQuery({ queryKey: ["settings"], queryFn: () => api.get_settings() });

	const [draft, set_draft] = useState<Settings | null>(null);
	const [status, set_status] = useState<SaveStatus>("idle");

	// The latest draft, kept in a ref so debounced saves never read stale state.
	const draft_ref = useRef<Settings | null>(null);
	// Seed the editable draft exactly once so live cache updates never clobber edits.
	const seeded = useRef(false);
	// The MCP values the app launched with, to detect when a restart is needed.
	const launch_mcp = useRef<{ enabled: boolean; port: number } | null>(null);
	// Pending debounce timer handle.
	const save_timer = useRef<number | null>(null);

	useEffect(() =>
	{
		if (settings_query.data && !seeded.current)
		{
			seeded.current = true;
			draft_ref.current = settings_query.data;
			launch_mcp.current = {
				enabled: settings_query.data.mcp_server_enabled,
				port: settings_query.data.mcp_server_port,
			};
			set_draft(settings_query.data);
		}
	}, [settings_query.data]);

	// Cancel any pending save when leaving the screen.
	useEffect(() =>
	{
		return () =>
		{
			if (save_timer.current !== null)
			{
				window.clearTimeout(save_timer.current);
			}
		};
	}, []);

	// About metadata and the lightweight update check.
	const app_info_query = useQuery({ queryKey: ["app_info"], queryFn: () => api.app_info() });
	const redirect_uris_query = useQuery({
		queryKey: ["hmrc_redirect_uris"],
		queryFn: () => api.hmrc_redirect_uris(),
	});
	const [update_check, set_update_check] = useState<UpdateCheck | null>(null);
	// Guards so the auto-check runs once and auto-update opens the page once.
	const auto_checked = useRef(false);
	const auto_update_opened = useRef(false);

	const check_mutation = useMutation({
		mutationFn: () => api.check_latest_version(),
		onSuccess: (result) =>
		{
			set_update_check(result);
			// In lightweight mode "auto-update" opens the release page to download.
			if (
				result.update_available &&
				draft_ref.current?.auto_update &&
				!auto_update_opened.current
			)
			{
				auto_update_opened.current = true;
				void openUrl(result.release_url);
			}
		},
		onError: (error) => notify_error(error),
	});

	// Auto-check once after load when the user has it enabled.
	useEffect(() =>
	{
		if (draft && draft.auto_check_for_updates && !auto_checked.current)
		{
			auto_checked.current = true;
			check_mutation.mutate();
		}
	}, [draft]);

	// HMRC businesses fetched on demand to populate the Business ID picker.
	const [businesses, set_businesses] = useState<HmrcBusiness[] | null>(null);
	const fetch_businesses_mutation = useMutation({
		mutationFn: () => api.hmrc_list_businesses(),
		onSuccess: (list) =>
		{
			set_businesses(list);
			// One business: select it outright; several: let the user choose below.
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

	// A readable label for one business in the picker.
	const business_label = (business: HmrcBusiness): string =>
	{
		const name = business.trading_name || business.type_of_business || "business";
		return `${name} — ${business.business_id}`;
	};

	// Persist the given settings, immediately or after the debounce window. On
	// success the settings cache is updated in place so the app root re-applies
	// appearance live, and HMRC status refreshes.
	const persist = (next: Settings, immediate: boolean) =>
	{
		if (save_timer.current !== null)
		{
			window.clearTimeout(save_timer.current);
			save_timer.current = null;
		}

		const run = () =>
		{
			set_status("saving");
			api
				.update_settings(next)
				.then((saved) =>
				{
					query_client.setQueryData(["settings"], saved);
					void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });
					set_status("saved");
				})
				.catch((error) =>
				{
					notify_error(error);
					set_status("idle");
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

	// Apply a top-level patch to the draft and schedule its save.
	const update = (patch: Partial<Settings>, immediate: boolean) =>
	{
		const next = { ...(draft_ref.current as Settings), ...patch };
		draft_ref.current = next;
		set_draft(next);
		persist(next, immediate);
	};

	// Apply a patch to the nested HMRC settings and schedule its save.
	const update_hmrc = (patch: Partial<Settings["hmrc"]>, immediate: boolean) =>
	{
		const current = draft_ref.current as Settings;
		const next = { ...current, hmrc: { ...current.hmrc, ...patch } };
		draft_ref.current = next;
		set_draft(next);
		persist(next, immediate);
	};

	// Restart the app to apply MCP changes (everything is already saved).
	const restart_now = () =>
	{
		if (window.confirm("Restart MyOpenUKTaxApp now to apply the MCP server changes?"))
		{
			void api.restart_app();
		}
	};

	if (settings_query.isLoading || !draft)
	{
		return <Spinner label="Loading settings…" />;
	}

	// A restart is needed only once the MCP settings differ from launch values.
	const mcp_restart_needed =
		launch_mcp.current !== null &&
		(draft.mcp_server_enabled !== launch_mcp.current.enabled ||
			draft.mcp_server_port !== launch_mcp.current.port);

	return (
		<div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
			<SaveIndicator status={status} />

			<Card>
				<CardHeader>
					<CardTitle>Appearance</CardTitle>
				</CardHeader>
				<CardContent className="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div className="flex flex-col gap-1.5">
						<Label htmlFor="theme">Colour theme</Label>
						<Select
							id="theme"
							title="System follows your operating system's light/dark mode"
							value={draft.theme}
							onChange={(event) => update({ theme: event.target.value }, true)}
						>
							{THEME_OPTIONS.map((option) => (
								<option key={option} value={option}>
									{option}
								</option>
							))}
						</Select>
					</div>
					<div className="flex flex-col gap-1.5">
						<Label htmlFor="font_size">Font size</Label>
						<Select
							id="font_size"
							title="Scales the whole interface"
							value={draft.font_size}
							onChange={(event) => update({ font_size: event.target.value }, true)}
						>
							{FONT_SIZE_OPTIONS.map((option) => (
								<option key={option} value={option}>
									{option}
								</option>
							))}
						</Select>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Data & logs</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					<div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
					<NumberField
						id="backups_pruned_after_days"
						label="Keep backups (days)"
						hint="Database backups older than this are pruned"
						value={draft.backups_pruned_after_days}
						on_change={(value) => update({ backups_pruned_after_days: value }, false)}
					/>
					<NumberField
						id="logs_pruned_after_days"
						label="Keep logs (days)"
						hint="Log files older than this are pruned (re-pruned immediately on change)"
						value={draft.logs_pruned_after_days}
						on_change={(value) => update({ logs_pruned_after_days: value }, false)}
					/>
					<NumberField
						id="backup_min_interval_seconds"
						label="Min backup interval (s)"
						hint="Smallest gap between automatic pre-write backups"
						value={draft.backup_min_interval_seconds}
						on_change={(value) => update({ backup_min_interval_seconds: value }, false)}
					/>
					</div>

					<div className="flex flex-wrap gap-2">
						<Button
							variant="outline"
							title="Open the Data folder in your file explorer"
							onClick={() => api.open_data_directory().catch(notify_error)}
						>
							<Icon name="folder_open" /> Open data directory
						</Button>
						<Button
							variant="outline"
							title="Open the Logs folder in your file explorer"
							onClick={() => api.open_logs_directory().catch(notify_error)}
						>
							<Icon name="folder_open" /> Open logs directory
						</Button>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>MCP server</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					<div className="flex flex-col gap-4 sm:flex-row sm:items-end">
						<label
							className="flex items-center gap-2 text-sm"
							title="Expose a local MCP server for AI agents while the app runs"
						>
							<input
								type="checkbox"
								className="h-4 w-4"
								checked={draft.mcp_server_enabled}
								onChange={(event) => update({ mcp_server_enabled: event.target.checked }, true)}
							/>
							Enable MCP server
						</label>
						<NumberField
							id="mcp_server_port"
							label="Port"
							hint="Localhost port for the MCP server"
							value={draft.mcp_server_port}
							on_change={(value) => update({ mcp_server_port: value }, false)}
						/>
					</div>

					{mcp_restart_needed ? (
						<div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-muted/50 px-3 py-2 text-sm">
							<span className="flex items-center gap-2 text-muted-foreground">
								<Icon name="restart_alt" className="text-base" />
								Saved — the MCP server only changes when the app restarts.
							</span>
							<Button
								size="sm"
								variant="secondary"
								title="Restart the app now to apply the MCP server changes"
								onClick={restart_now}
							>
								<Icon name="restart_alt" /> Restart now
							</Button>
						</div>
					) : null}
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>HMRC connection</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="environment">Environment</Label>
							<Select
								id="environment"
								title="Use the sandbox for testing, production for real submissions"
								value={draft.hmrc.environment}
								onChange={(event) => update_hmrc({ environment: event.target.value }, true)}
							>
								{HMRC_ENVIRONMENT_OPTIONS.map((option) => (
									<option key={option} value={option}>
										{option}
									</option>
								))}
							</Select>
						</div>
						<TextField id="client_id" label="Client ID" value={draft.hmrc.client_id} on_change={(value) => update_hmrc({ client_id: value }, false)} hint="From your HMRC Developer Hub application" />
						<TextField id="client_secret" label="Client secret" type="password" value={draft.hmrc.client_secret} on_change={(value) => update_hmrc({ client_secret: value }, false)} hint="Stored locally in the settings file" />
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="national_insurance_number">National Insurance no.</Label>
							<Input
								id="national_insurance_number"
								title="Your National Insurance number (e.g. AB123456C)"
								placeholder="AB123456C"
								value={draft.hmrc.national_insurance_number}
								onChange={(event) =>
									update_hmrc({ national_insurance_number: normalize_nino(event.target.value) }, false)
								}
							/>
							{draft.hmrc.national_insurance_number.length > 0 &&
							!is_valid_nino(draft.hmrc.national_insurance_number) ? (
								<span className="text-xs text-destructive">
									Doesn’t look like a NINO yet (expected like AB123456C).
								</span>
							) : null}
						</div>
						<TextField id="business_id" label="Business ID" value={draft.hmrc.business_id} on_change={(value) => update_hmrc({ business_id: value }, false)} hint="HMRC business ID (e.g. XAIS…), not your UTR. Use Fetch below to look it up." />
					</div>

					{/* Sandbox only: select a stubbed HMRC response. */}
					{draft.hmrc.environment === "sandbox" ? (
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="gov_test_scenario">Sandbox test scenario (optional)</Label>
							<Input
								id="gov_test_scenario"
								title="Sent as the Gov-Test-Scenario header to select a stubbed HMRC sandbox response"
								placeholder="e.g. a scenario name from HMRC's API docs"
								value={draft.hmrc.gov_test_scenario}
								onChange={(event) => update_hmrc({ gov_test_scenario: event.target.value }, false)}
							/>
							<span className="text-xs text-muted-foreground">
								Only used in the sandbox — sent as the Gov-Test-Scenario header so HMRC
								returns canned data (see the HMRC API docs for scenario names).
							</span>
						</div>
					) : null}

					{/* The loopback redirect URIs to register on the HMRC Developer Hub. */}
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

					{/* Look up the Business ID from HMRC instead of typing it. */}
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
									value={draft.hmrc.business_id}
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

						{businesses && businesses.length === 0 ? (
							<p className="text-sm text-muted-foreground">
								No businesses found on your HMRC record.
							</p>
						) : null}
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Updates</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4">
					<InfoRow label="Current version" value={`v${app_info_query.data?.version ?? "…"}`} />

					<label className="flex items-center gap-2 text-sm" title="Check GitHub for a newer release on startup">
						<input
							type="checkbox"
							className="h-4 w-4"
							checked={draft.auto_check_for_updates}
							onChange={(event) => update({ auto_check_for_updates: event.target.checked }, true)}
						/>
						Automatically check for updates
					</label>
					<label className="flex items-center gap-2 text-sm" title="When a newer release is found, open it to download automatically">
						<input
							type="checkbox"
							className="h-4 w-4"
							checked={draft.auto_update}
							onChange={(event) => update({ auto_update: event.target.checked }, true)}
						/>
						Automatically download updates when available
					</label>

					<div className="flex flex-wrap items-center gap-2">
						<Button
							variant="outline"
							title="Check GitHub now for a newer release"
							disabled={check_mutation.isPending}
							onClick={() => check_mutation.mutate()}
						>
							<Icon name="search" /> Check now
						</Button>
						<Button
							title="Open the latest release to download and install"
							disabled={!update_check?.update_available}
							onClick={() => update_check && void openUrl(update_check.release_url)}
						>
							<Icon name="download" /> Update now
						</Button>
						{check_mutation.isPending ? <Spinner label="Checking…" /> : null}
					</div>

					{update_check ? (
						<div className="text-sm">
							{update_check.latest_version === "" ? (
								<span className="text-muted-foreground">No published releases found yet.</span>
							) : update_check.update_available ? (
								<span className="flex items-center gap-1 text-income">
									<Icon name="new_releases" className="text-base" /> Update available: v
									{update_check.latest_version}
								</span>
							) : (
								<span className="flex items-center gap-1 text-muted-foreground">
									<Icon name="check_circle" className="text-base" /> Up to date (latest v
									{update_check.latest_version})
								</span>
							)}
						</div>
					) : null}
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>About</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-2">
					<InfoRow label="Application" value={app_info_query.data?.name ?? ""} />
					<InfoRow label="Author" value={app_info_query.data?.authors ?? ""} />
					<div className="flex items-center justify-between gap-2 text-sm">
						<span className="text-muted-foreground">Homepage</span>
						<button
							className="truncate text-primary underline-offset-2 hover:underline"
							title="Open the homepage in your default browser"
							onClick={() =>
								app_info_query.data && void openUrl(app_info_query.data.homepage)
							}
						>
							{app_info_query.data?.homepage}
						</button>
					</div>
					<InfoRow label="License" value={app_info_query.data?.license ?? ""} />
				</CardContent>
			</Card>
		</div>
	);
}

// A simple label/value row used by the About and Updates sections.
function InfoRow({ label, value }: { label: string; value: string })
{
	return (
		<div className="flex items-center justify-between gap-2 text-sm">
			<span className="text-muted-foreground">{label}</span>
			<span className="font-medium">{value}</span>
		</div>
	);
}

// A small, right-aligned auto-save status indicator.
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

// A labelled numeric input that reports back a clamped non-negative integer.
function NumberField(props: {
	id: string;
	label: string;
	hint: string;
	value: number;
	on_change: (value: number) => void;
})
{
	return (
		<div className="flex flex-col gap-1.5">
			<Label htmlFor={props.id}>{props.label}</Label>
			<Input
				id={props.id}
				type="number"
				min="0"
				title={props.hint}
				value={props.value}
				onChange={(event) => props.on_change(Math.max(0, Math.floor(Number(event.target.value) || 0)))}
			/>
		</div>
	);
}

// A labelled text/password input.
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
