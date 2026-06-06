// Renders the transient notifications held in the notify store. Mounted once at
// the app root; toasts auto-dismiss via the store's timer.

import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import { use_notify, type NotifyKind } from "@/store/notify";

// Per-kind colour and icon mapping.
const kind_styles: Record<NotifyKind, { className: string; icon: string }> = {
	info: { className: "bg-secondary text-secondary-foreground", icon: "info" },
	success: { className: "bg-income text-income-foreground", icon: "check_circle" },
	error: { className: "bg-destructive text-destructive-foreground", icon: "error" },
};

export function Toaster()
{
	const messages = use_notify((state) => state.messages);
	const dismiss = use_notify((state) => state.dismiss);

	return (
		<div className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2">
			{messages.map((message) =>
			{
				const style = kind_styles[message.kind];
				return (
					<div
						key={message.id}
						className={cn(
							"pointer-events-auto flex items-start gap-2 rounded-md px-3 py-2 text-sm shadow-lg",
							style.className,
						)}
						role="status"
					>
						<Icon name={style.icon} className="text-base" />
						<span className="flex-1 break-words">{message.text}</span>
						<button
							className="opacity-80 hover:opacity-100"
							title="Dismiss"
							onClick={() => dismiss(message.id)}
						>
							<Icon name="close" className="text-base" />
						</button>
					</div>
				);
			})}
		</div>
	);
}
