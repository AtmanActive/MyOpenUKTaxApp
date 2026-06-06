// Applies the user's appearance settings to the document.
//
// Theme: toggles the `.dark` class on <html>. When the theme is "system" it
// follows the OS colour scheme and keeps following it live via matchMedia.
// Font size: the nine named sizes map to a root font-size in px; because all
// Tailwind sizing is rem-based, changing the root scales the entire UI.

import { useEffect } from "react";

// Named font sizes -> root font-size in pixels.
const FONT_SIZE_PX: Record<string, number> = {
	"xxx-small": 10,
	"xx-small": 11,
	"x-small": 12,
	small: 13,
	medium: 14,
	large: 16,
	"x-large": 18,
	"xx-large": 20,
	"xxx-large": 24,
};

export function use_appearance(theme: string, font_size: string): void
{
	// Apply and (for "system") track the colour theme.
	useEffect(() =>
	{
		const root = document.documentElement;
		const media = window.matchMedia("(prefers-color-scheme: dark)");

		const apply_theme = () =>
		{
			const is_dark = theme === "dark" || (theme === "system" && media.matches);
			root.classList.toggle("dark", is_dark);
		};

		apply_theme();

		// Only subscribe to OS changes while we are following the system theme.
		if (theme === "system")
		{
			media.addEventListener("change", apply_theme);
			return () => media.removeEventListener("change", apply_theme);
		}
	}, [theme]);

	// Apply the root font size that scales the whole interface.
	useEffect(() =>
	{
		const pixels = FONT_SIZE_PX[font_size] ?? FONT_SIZE_PX.medium;
		document.documentElement.style.fontSize = `${pixels}px`;
	}, [font_size]);
}
