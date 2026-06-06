// Form label primitive (shadcn/ui-style).

import type { LabelHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Label({ className, ...rest }: LabelHTMLAttributes<HTMLLabelElement>)
{
	return (
		<label
			className={cn("text-sm font-medium leading-none text-foreground", className)}
			{...rest}
		/>
	);
}
