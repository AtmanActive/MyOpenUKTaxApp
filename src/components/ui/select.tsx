// Dropdown primitive built on the native <select> element (styled to match the
// theme). Native selects are fully accessible and keyboard-friendly out of the
// box, which keeps the dependency surface small. Call sites pass <option>s.

import { forwardRef, type SelectHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
	function Select({ className, children, ...rest }, ref)
	{
		return (
			<select
				ref={ref}
				className={cn(
					"flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
					className,
				)}
				{...rest}
			>
				{children}
			</select>
		);
	},
);
