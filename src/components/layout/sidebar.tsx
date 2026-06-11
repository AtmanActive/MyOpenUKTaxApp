// The always-present main navigation. Renders the seven section icons plus an
// Exit control. Laid out vertically on the left in landscape, and as a
// horizontal taskbar along the bottom in portrait orientation.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import { SECTIONS, use_app_store, type HmrcConnection, type SectionId } from "@/store/app-store";

interface SidebarProps
{
	orientation: "vertical" | "horizontal";
}

export function Sidebar({ orientation }: SidebarProps)
{
	const active_section = use_app_store((state) => state.active_section);
	const set_active_section = use_app_store((state) => state.set_active_section);
	const new_event = use_app_store((state) => state.new_event);
	const hmrc_connection = use_app_store((state) => state.hmrc_connection);

	const is_vertical = orientation === "vertical";

	// Navigate to a section; the Add Event entry always opens a blank form.
	const go_to = (id: SectionId) =>
	{
		if (id === "add-event")
		{
			new_event();
		}
		else
		{
			set_active_section(id);
		}
	};

	// Close the application window (the spec's Exit action).
	const exit_app = () =>
	{
		void getCurrentWindow().close();
	};

	return (
		<nav
			className={cn(
				"flex gap-1 border-border bg-card",
				is_vertical
					? "h-full w-56 flex-col border-r p-2"
					: "w-full flex-row items-stretch overflow-x-auto border-t p-1",
			)}
			aria-label="Main navigation"
		>
			<div className={cn("flex gap-1", is_vertical ? "flex-col" : "flex-1 flex-row")}>
				{SECTIONS.map((section) =>
				{
					const is_active = active_section === section.id;
					return (
						<button
							key={section.id}
							title={section.hint}
							onClick={() => go_to(section.id)}
							className={cn(
								"flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
								is_vertical ? "w-full justify-start" : "flex-1 flex-col justify-center gap-1 text-xs",
								is_active
									? "bg-primary text-primary-foreground"
									: "text-foreground hover:bg-accent hover:text-accent-foreground",
							)}
						>
							{section.id === "dashboard" ? (
									<img
										src="/logo_shadow.png"
										alt=""
										aria-hidden="true"
										className="h-6 w-6 shrink-0 object-contain"
									/>
								) : (
									<Icon name={section.icon} className="text-xl" />
								)}
							<span className={cn(!is_vertical && "text-[0.65rem]")}>{section.label}</span>
							{section.id === "hmrc-connection" ? (
									<ConnectionLed state={hmrc_connection} vertical={is_vertical} />
								) : null}
						</button>
					);
				})}
			</div>

			{/* Exit lives at the far end (bottom in vertical, right in horizontal). */}
			<button
				title="Exit MyOpenUKTaxApp"
				onClick={exit_app}
				className={cn(
					"flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground",
					is_vertical ? "mt-auto w-full justify-start" : "flex-col justify-center gap-1 text-xs",
				)}
			>
				<Icon name="logout" className="text-xl" />
				<span className={cn(!is_vertical && "text-[0.65rem]")}>Exit</span>
			</button>
		</nav>
	);
}

// A small round LED reflecting the HMRC connection state next to its menu item.
function ConnectionLed({ state, vertical }: { state: HmrcConnection; vertical: boolean })
{
	const colour =
		state === "connected"
			? "bg-green-500"
			: state === "failed"
				? "bg-red-500"
				: state === "connecting"
					? "bg-cyan-400 animate-pulse"
					: "bg-gray-400";
	const label =
		state === "connected"
			? "Connected to HMRC"
			: state === "failed"
				? "HMRC connection failed"
				: state === "connecting"
					? "Connecting to HMRC…"
					: "HMRC not connected";
	return (
		<span
			title={label}
			aria-label={label}
			className={cn("h-2.5 w-2.5 shrink-0 rounded-full ring-1 ring-black/20", colour, vertical && "ml-auto")}
		/>
	);
}
