// Small styling helper shared by every UI primitive. `cn` merges conditional
// class names (clsx) and then de-duplicates conflicting Tailwind utilities
// (tailwind-merge) so component variants can be overridden cleanly.
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[])
{
	return twMerge(clsx(inputs));
}
