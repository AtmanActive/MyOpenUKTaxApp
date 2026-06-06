// Frontend entry point.
//
// Loads the Material Symbols icon font and the global stylesheet, sets up the
// React Query client used by every section to talk to the Tauri commands, then
// mounts the app.

import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
// Only the "outlined" Material Symbols variant is used; importing just that CSS
// avoids bundling the rounded and sharp font files we never reference.
import "material-symbols/outlined.css";
import "./index.css";
import App from "./App";

// One shared query client. Commands are local IPC calls, so retries are off and
// data is considered fresh briefly to avoid redundant refetches.
const query_client = new QueryClient({
	defaultOptions: {
		queries: {
			retry: false,
			staleTime: 2000,
			refetchOnWindowFocus: false,
		},
	},
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<QueryClientProvider client={query_client}>
			<App />
		</QueryClientProvider>
	</React.StrictMode>,
);
