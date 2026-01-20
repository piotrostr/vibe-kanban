import { open } from "@tauri-apps/plugin-shell";

/**
 * Check for Tauri at call time, not module load time.
 * Tauri v2 uses __TAURI_INTERNALS__ regardless of withGlobalTauri setting.
 */
function isTauri(): boolean {
	return "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
}

/**
 * Opens a URL in the system's default browser.
 * Uses Tauri's shell plugin when running in Tauri, falls back to window.open otherwise.
 */
export async function openExternal(url: string): Promise<void> {
	console.log("[openExternal] url:", url, "isTauri:", isTauri());
	if (isTauri()) {
		try {
			console.log("[openExternal] calling Tauri open()");
			await open(url);
			console.log("[openExternal] Tauri open() succeeded");
		} catch (error) {
			console.error("[openExternal] Tauri open() failed:", error);
			window.open(url, "_blank", "noopener,noreferrer");
		}
	} else {
		console.log("[openExternal] using window.open fallback");
		window.open(url, "_blank", "noopener,noreferrer");
	}
}
