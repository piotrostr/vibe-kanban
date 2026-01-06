/// <reference lib="webworker" />
import { cleanupOutdatedCaches, precacheAndRoute } from "workbox-precaching";
import { clientsClaim } from "workbox-core";
import { registerRoute, NavigationRoute } from "workbox-routing";
import { CacheFirst } from "workbox-strategies";
import { ExpirationPlugin } from "workbox-expiration";
import { CacheableResponsePlugin } from "workbox-cacheable-response";
import { createHandlerBoundToURL } from "workbox-precaching";

declare let self: ServiceWorkerGlobalScope & {
	__WB_MANIFEST: Array<{ url: string; revision: string | null }>;
};

// Take control immediately
self.skipWaiting();
clientsClaim();

// Precache assets - the manifest is injected by vite-plugin-pwa
precacheAndRoute(self.__WB_MANIFEST);

// Clean up old caches
cleanupOutdatedCaches();

// Handle navigation requests with app shell
registerRoute(new NavigationRoute(createHandlerBoundToURL("index.html")));

// Cache Google Fonts
registerRoute(
	/^https:\/\/fonts\.googleapis\.com\/.*/i,
	new CacheFirst({
		cacheName: "google-fonts-cache",
		plugins: [
			new ExpirationPlugin({
				maxEntries: 10,
				maxAgeSeconds: 60 * 60 * 24 * 365, // 1 year
			}),
			new CacheableResponsePlugin({
				statuses: [0, 200],
			}),
		],
	}),
	"GET",
);

// Handle notification clicks - focus existing window or open new one
self.addEventListener("notificationclick", (event) => {
	event.notification.close();

	const urlToOpen = event.notification.data?.url || "/";

	event.waitUntil(
		self.clients
			.matchAll({ type: "window", includeUncontrolled: true })
			.then((clientList) => {
				// Try to find an existing window to focus
				for (const client of clientList) {
					// Check if client URL is from the same origin
					if (client.url.startsWith(self.location.origin)) {
						// Focus the existing window and navigate to the target URL
						return client.focus().then((focusedClient) => {
							if (focusedClient) {
								focusedClient.postMessage({
									type: "NOTIFICATION_CLICK",
									url: urlToOpen,
								});
							}
							return focusedClient;
						});
					}
				}
				// No existing window found - open a new one
				return self.clients.openWindow(urlToOpen);
			}),
	);
});
