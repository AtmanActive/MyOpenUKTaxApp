// A small inline loading indicator used while command queries are in flight.

import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";

export function Spinner({ className, label }: { className?: string; label?: string })
{
	return (
		<div className={cn("flex items-center gap-2 text-muted-foreground", className)}>
			<Icon name="progress_activity" className="animate-spin" />
			{label ? <span className="text-sm">{label}</span> : null}
		</div>
	);
}
