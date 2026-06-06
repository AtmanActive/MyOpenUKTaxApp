// Material Symbols icon wrapper.
//
// The Material Symbols font is loaded once in main.tsx, giving access to every
// Material icon by name. This component renders one glyph by ligature name.

import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

interface IconProps extends HTMLAttributes<HTMLSpanElement>
{
	name: string;
}

export function Icon({ name, className, ...rest }: IconProps)
{
	return (
		<span className={cn("material-symbols-outlined leading-none", className)} aria-hidden="true" {...rest}>
			{name}
		</span>
	);
}
