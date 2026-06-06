// The application shell: the always-present sidebar and topbar around a main
// pane that fills the remaining space. In portrait orientation the sidebar
// moves to the bottom as a horizontal taskbar, per the overview spec.

import { Sidebar } from "@/components/layout/sidebar";
import { Topbar } from "@/components/layout/topbar";
import { cn } from "@/lib/utils";
import { use_is_portrait } from "@/lib/use-is-portrait";
import { use_app_store, type SectionId } from "@/store/app-store";
import { AddEventSection } from "@/sections/add-event";
import { CategoryMappingSection } from "@/sections/category-mapping";
import { DashboardSection } from "@/sections/dashboard";
import { EventsSection } from "@/sections/events";
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
		case "hmrc":
			return <HmrcHistorySection />;
		case "settings":
			return <SettingsSection />;
	}
}

export function AppShell()
{
	const active_section = use_app_store((state) => state.active_section);
	const is_portrait = use_is_portrait();

	return (
		<div className={cn("flex h-full w-full overflow-hidden", is_portrait ? "flex-col" : "flex-row")}>
			{!is_portrait ? <Sidebar orientation="vertical" /> : null}

			<div className="flex min-w-0 flex-1 flex-col">
				<Topbar />
				<main className="flex-1 overflow-auto p-4">{render_section(active_section)}</main>
			</div>

			{is_portrait ? <Sidebar orientation="horizontal" /> : null}
		</div>
	);
}
