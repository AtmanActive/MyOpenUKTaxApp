// Table primitives (shadcn/ui-style). The recorded-events screen needs sortable,
// scrollable tables, so these wrap the native table elements with theme styling
// while leaving sorting behaviour to the calling section.

import type { HTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Table({ className, ...rest }: HTMLAttributes<HTMLTableElement>)
{
	return (
		<div className="relative w-full overflow-auto">
			<table className={cn("w-full caption-bottom text-sm", className)} {...rest} />
		</div>
	);
}

export function TableHeader({ className, ...rest }: HTMLAttributes<HTMLTableSectionElement>)
{
	return <thead className={cn("[&_tr]:border-b", className)} {...rest} />;
}

export function TableBody({ className, ...rest }: HTMLAttributes<HTMLTableSectionElement>)
{
	return <tbody className={cn("[&_tr:last-child]:border-0", className)} {...rest} />;
}

export function TableRow({ className, ...rest }: HTMLAttributes<HTMLTableRowElement>)
{
	return (
		<tr
			className={cn("border-b border-border transition-colors hover:bg-muted/50", className)}
			{...rest}
		/>
	);
}

export function TableHead({ className, ...rest }: ThHTMLAttributes<HTMLTableCellElement>)
{
	return (
		<th
			className={cn(
				"h-10 px-3 text-left align-middle font-medium text-muted-foreground",
				className,
			)}
			{...rest}
		/>
	);
}

export function TableCell({ className, ...rest }: TdHTMLAttributes<HTMLTableCellElement>)
{
	return <td className={cn("p-3 align-middle", className)} {...rest} />;
}
