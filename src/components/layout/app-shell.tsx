// The application shell: the always-present sidebar and topbar around a main
// pane that fills the remaining space. In portrait orientation the sidebar
// moves to the bottom as a horizontal taskbar, per the overview spec.

import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Sidebar } from "@/components/layout/sidebar";
import { Topbar } from "@/components/layout/topbar";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { use_is_portrait } from "@/lib/use-is-portrait";
import { use_app_store, type SectionId } from "@/store/app-store";
import { AddEventSection } from "@/sections/add-event";
import { CategoryMappingSection } from "@/sections/category-mapping";
import { DashboardSection } from "@/sections/dashboard";
import { EventsSection } from "@/sections/events";
import { HmrcConnectionSection } from "@/sections/hmrc-connection";
import { HmrcGetSection } from "@/sections/hmrc-get";
import { HmrcHistorySection } from "@/sections/hmrc-history";
import { SettingsSection } from "@/sections/settings";
import { SubcategoriesSection } from "@/sections/subcategories";

// Map the active section id to its component.
function render_section(section: SectionId)
{
	switch (section)
	{
		case "dashboard":
			return <DashboardSection />;
		case "add-event":
			return <AddEventSection />;
		case "events":
			return <EventsSection />;
		case "subcategories":
			return <SubcategoriesSection />;
		case "mapping":
			return <CategoryMappingSection />;
		case "hmrc-connection":
			return <HmrcConnectionSection />;
		case "hmrc":
			return <HmrcHistorySection />;
		case "hmrc-get":
			return <HmrcGetSection />;
		case "settings":
			return <SettingsSection />;
	}
}

export function AppShell()
{
	const active_section = use_app_store((state) => state.active_section);
	const is_portrait = use_is_portrait();

	// Seed the sidebar HMRC LED from the stored token on launch: green if a token
	// is present, otherwise grey. The HMRC Connection screen drives it thereafter.
	const hmrc_connection = use_app_store((state) => state.hmrc_connection);
	const set_hmrc_connection = use_app_store((state) => state.set_hmrc_connection);
	const status_query = useQuery({ queryKey: ["hmrc_status"], queryFn: () => api.hmrc_status() });
	useEffect(() =>
	{
		if (hmrc_connection === "unknown" && status_query.data?.has_token)
		{
			set_hmrc_connection("connected");
		}
	}, [status_query.data, hmrc_connection, set_hmrc_connection]);

	// Seed the run mode from the backend on launch; the topbar toggle drives it
	// thereafter. The `mode-*` root class powers the runmode_* visibility CSS.
	const run_mode = use_app_store((state) => state.run_mode);
	const set_run_mode = use_app_store((state) => state.set_run_mode);
	const run_mode_query = useQuery({ queryKey: ["run_mode"], queryFn: () => api.get_run_mode() });
	useEffect(() =>
	{
		if (run_mode_query.data)
		{
			set_run_mode(run_mode_query.data);
		}
	}, [run_mode_query.data, set_run_mode]);

	return (
		<div
			className={cn(
				"flex h-full w-full overflow-hidden",
				`mode-${run_mode}`,
				is_portrait ? "flex-col" : "flex-row",
			)}
		>
			{!is_portrait ? <Sidebar orientation="vertical" /> : null}

			<div className="flex min-w-0 flex-1 flex-col">
				<Topbar />
				<main className="flex-1 overflow-auto p-4">{render_section(active_section)}</main>
			</div>

			{is_portrait ? <Sidebar orientation="horizontal" /> : null}
		</div>
	);
}
