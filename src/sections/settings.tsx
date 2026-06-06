// Section 7 - Settings.
//
// Appearance (theme + font size), data-retention windows, the embedded MCP
// server, and the HMRC connection credentials. Saving persists to the
// exe-adjacent settings JSON; the backend preserves backend-managed fields
// (device id, OAuth tokens). Appearance changes apply once the settings query
// is invalidated and re-read by the app root.

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
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
	type Settings,
} from "@/lib/types";
import { notify_error, use_notify } from "@/store/notify";

export function SettingsSection()
{
	const push = use_notify((state) => state.push);
	const query_client = useQueryClient();

	const settings_query = useQuery({ queryKey: ["settings"], queryFn: () => api.get_settings() });
	const [draft, set_draft] = useState<Settings | null>(null);

	// Seed the editable draft once the current settings arrive.
	useEffect(() =>
	{
		if (settings_query.data)
		{
			set_draft(settings_query.data);
		}
	}, [settings_query.data]);

	const save_mutation = useMutation({
		mutationFn: (settings: Settings) => api.update_settings(settings),
		onSuccess: () =>
		{
			push("success", "Settings saved.");
			// Re-read so the app root re-applies theme/font and HMRC status refreshes.
			void query_client.invalidateQueries({ queryKey: ["settings"] });
			void query_client.invalidateQueries({ queryKey: ["hmrc_status"] });
		},
		onError: (error) => notify_error(error),
	});

	if (settings_query.isLoading || !draft)
	{
		return <Spinner label="Loading settings…" />;
	}

	// Helpers to update nested draft fields immutably.
	const set_field = <K extends keyof Settings>(key: K, value: Settings[K]) =>
		set_draft((current) => (current ? { ...current, [key]: value } : current));
	const set_hmrc_field = <K extends keyof Settings["hmrc"]>(key: K, value: Settings["hmrc"][K]) =>
		set_draft((current) =>
			current ? { ...current, hmrc: { ...current.hmrc, [key]: value } } : current,
		);

	return (
		<div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
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
							onChange={(event) => set_field("theme", event.target.value)}
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
							onChange={(event) => set_field("font_size", event.target.value)}
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
				<CardContent className="grid grid-cols-1 gap-4 sm:grid-cols-3">
					<NumberField
						id="backups_pruned_after_days"
						label="Keep backups (days)"
						hint="Database backups older than this are pruned"
						value={draft.backups_pruned_after_days}
						on_change={(value) => set_field("backups_pruned_after_days", value)}
					/>
					<NumberField
						id="logs_pruned_after_days"
						label="Keep logs (days)"
						hint="Log files older than this are pruned"
						value={draft.logs_pruned_after_days}
						on_change={(value) => set_field("logs_pruned_after_days", value)}
					/>
					<NumberField
						id="backup_min_interval_seconds"
						label="Min backup interval (s)"
						hint="Smallest gap between automatic pre-write backups"
						value={draft.backup_min_interval_seconds}
						on_change={(value) => set_field("backup_min_interval_seconds", value)}
					/>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>MCP server</CardTitle>
				</CardHeader>
				<CardContent className="flex flex-col gap-4 sm:flex-row sm:items-end">
					<label className="flex items-center gap-2 text-sm" title="Expose a local MCP server for AI agents while the app runs">
						<input
							type="checkbox"
							className="h-4 w-4"
							checked={draft.mcp_server_enabled}
							onChange={(event) => set_field("mcp_server_enabled", event.target.checked)}
						/>
						Enable MCP server (applies on next launch)
					</label>
					<NumberField
						id="mcp_server_port"
						label="Port"
						hint="Localhost port for the MCP server"
						value={draft.mcp_server_port}
						on_change={(value) => set_field("mcp_server_port", value)}
					/>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>HMRC connection</CardTitle>
				</CardHeader>
				<CardContent className="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div className="flex flex-col gap-1.5">
						<Label htmlFor="environment">Environment</Label>
						<Select
							id="environment"
							title="Use the sandbox for testing, production for real submissions"
							value={draft.hmrc.environment}
							onChange={(event) => set_hmrc_field("environment", event.target.value)}
						>
							{HMRC_ENVIRONMENT_OPTIONS.map((option) => (
								<option key={option} value={option}>
									{option}
								</option>
							))}
						</Select>
					</div>
					<TextField id="client_id" label="Client ID" value={draft.hmrc.client_id} on_change={(value) => set_hmrc_field("client_id", value)} hint="From your HMRC Developer Hub application" />
					<TextField id="client_secret" label="Client secret" type="password" value={draft.hmrc.client_secret} on_change={(value) => set_hmrc_field("client_secret", value)} hint="Stored locally in the settings file" />
					<TextField id="redirect_uri" label="Redirect URI" value={draft.hmrc.redirect_uri} on_change={(value) => set_hmrc_field("redirect_uri", value)} hint="Must match the redirect registered with HMRC" />
					<TextField id="national_insurance_number" label="National Insurance no." value={draft.hmrc.national_insurance_number} on_change={(value) => set_hmrc_field("national_insurance_number", value)} hint="Your NINO (e.g. AA123456A)" />
					<TextField id="business_id" label="Business ID" value={draft.hmrc.business_id} on_change={(value) => set_hmrc_field("business_id", value)} hint="Your MTD self-employment business id" />
				</CardContent>
			</Card>

			<div className="flex justify-end">
				<Button title="Save all settings" disabled={save_mutation.isPending} onClick={() => save_mutation.mutate(draft)}>
					<Icon name="save" /> Save settings
				</Button>
			</div>
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
