// Text/number/date input primitive (shadcn/ui-style). Native input types are
// used directly so the spec's number spinners and date pickers come for free.

import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
	function Input({ className, ...rest }, ref)
	{
		return (
			<input
				ref={ref}
				className={cn(
					"flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
					className,
				)}
				{...rest}
			/>
		);
	},
);
