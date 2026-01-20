import { open } from "@tauri-apps/plugin-shell";

// Tauri v2 uses __TAURI_INTERNALS__ regardless of withGlobalTauri setting
const isTauri = "__TAURI_INTERNALS__" in window || "__TAURI__" in window;

/**
 * Opens a URL in the system's default browser.
 * Uses Tauri's shell plugin when running in Tauri, falls back to window.open otherwise.
 */
export async function openExternal(url: string): Promise<void> {
	if (isTauri) {
		try {
			await open(url);
		} catch (error) {
			console.error("Failed to open URL via Tauri shell:", error);
			window.open(url, "_blank", "noopener,noreferrer");
		}
	} else {
		window.open(url, "_blank", "noopener,noreferrer");
	}
}
