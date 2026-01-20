import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./styles/index.css";
import { ClickToComponent } from "click-to-react-component";
import { VibeKanbanWebCompanion } from "vibe-kanban-web-companion";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
// Import modal type definitions
import "./types/modals";
import { openExternal } from "./lib/openExternal";

const isTauri = "__TAURI_INTERNALS__" in window || "__TAURI__" in window;

// Intercept external link clicks in Tauri to open in system browser
if (isTauri) {
	document.addEventListener("click", (e) => {
		const target = e.target as HTMLElement;
		const anchor = target.closest("a");
		if (!anchor) return;

		const href = anchor.getAttribute("href");
		if (!href) return;

		// Only intercept external URLs (http/https)
		if (href.startsWith("http://") || href.startsWith("https://")) {
			e.preventDefault();
			void openExternal(href);
		}
	});
}

// Dev helper for testing Tauri notifications
if (isTauri) {
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
