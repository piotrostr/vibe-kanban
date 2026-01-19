import { open } from "@tauri-apps/plugin-shell";

const isTauri = "__TAURI__" in window;

/**
 * Opens a URL in the system's default browser.
 * Uses Tauri's shell plugin when running in Tauri, falls back to window.open otherwise.
 */
export async function openExternal(url: string): Promise<void> {
	if (isTauri) {
		await open(url);
	} else {
		window.open(url, "_blank", "noopener,noreferrer");
	}
}
