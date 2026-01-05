import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./styles/index.css";
import { ClickToComponent } from "click-to-react-component";
import { VibeKanbanWebCompanion } from "vibe-kanban-web-companion";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
// Import modal type definitions
import "./types/modals";

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
