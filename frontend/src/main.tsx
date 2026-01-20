import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./styles/index.css";
import { ClickToComponent } from "click-to-react-component";
import { VibeKanbanWebCompanion } from "vibe-kanban-web-companion";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
// Import modal type definitions
import "./types/modals";

// Dev helper for testing Tauri notifications
if ("__TAURI_INTERNALS__" in window || "__TAURI__" in window) {
	import("@tauri-apps/plugin-notification").then(
		({ sendNotification, isPermissionGranted, requestPermission }) => {
			(window as unknown as Record<string, unknown>).testNotification =
				async () => {
					let granted = await isPermissionGranted();
					if (!granted) granted = (await requestPermission()) === "granted";
					if (granted)
						sendNotification({ title: "Test", body: "Notification working!" });
				};
			console.log("Tauri detected - run testNotification() to test");
		},
	);
}

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 1000 * 60 * 5, // 5 minutes
			refetchOnWindowFocus: false,
		},
	},
});

ReactDOM.createRoot(document.getElementById("root")!).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<ClickToComponent />
			<VibeKanbanWebCompanion />
			<App />
			{/*<TanStackDevtools plugins={[FormDevtoolsPlugin()]} />*/}
			{/* <ReactQueryDevtools initialIsOpen={false} /> */}
		</QueryClientProvider>
	</React.StrictMode>,
);
