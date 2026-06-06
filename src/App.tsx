// Application root.
//
// Reads the current settings and applies the appearance (theme + font size) to
// the document, then renders the shell and the toast host. The settings query
// shares its cache key with the Settings screen, so saving there re-applies the
// appearance here automatically.

import { useQuery } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/app-shell";
import { Toaster } from "@/components/ui/toaster";
import { use_appearance } from "@/components/theme/use-appearance";
import { api } from "@/lib/api";

export default function App()
{
	const settings_query = useQuery({ queryKey: ["settings"], queryFn: () => api.get_settings() });

	// Fall back to sensible defaults until the settings have loaded.
	const theme = settings_query.data?.theme ?? "system";
	const font_size = settings_query.data?.font_size ?? "medium";
	use_appearance(theme, font_size);

	return (
		<>
			<AppShell />
			<Toaster />
		</>
	);
}
