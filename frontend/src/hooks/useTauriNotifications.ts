import { useEffect, useRef, useCallback, useState } from "react";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
} from "@tauri-apps/plugin-notification";
import type { TaskWithAttemptStatus } from "shared/types";

// Tauri v2 uses __TAURI_INTERNALS__ regardless of withGlobalTauri setting
const isTauri = "__TAURI_INTERNALS__" in window || "__TAURI__" in window;

/**
 * Hook to show native Tauri notifications when tasks complete (status changes to 'inreview')
 *
 * Note: On desktop (macOS/Windows/Linux), notification click handling is not supported
 * by the Tauri notification plugin. Clicking a notification will bring the app to
 * foreground but won't navigate to a specific task. This is a limitation of the
 * underlying notify-rust library. The onAction API only works on mobile platforms.
 */
export const useTauriNotifications = (
	tasks: TaskWithAttemptStatus[],
	_projectId: string | undefined,
) => {
	const prevTasksRef = useRef<Map<string, string>>(new Map());
	const [permissionGranted, setPermissionGranted] = useState(false);

	// Request permission on mount
	useEffect(() => {
		if (!isTauri) return;

		(async () => {
			let granted = await isPermissionGranted();
			if (!granted) {
				const permission = await requestPermission();
				granted = permission === "granted";
			}
			setPermissionGranted(granted);
		})();
	}, []);

	const showNotification = useCallback(
		async (title: string, body: string) => {
			if (!isTauri || !permissionGranted) return;

			try {
				sendNotification({ title, body });
			} catch (error) {
				console.error("Failed to send notification:", error);
			}
		},
		[permissionGranted],
	);

	// Watch for task status changes to 'inreview'
	useEffect(() => {
		if (!permissionGranted) return;

		const prevTasks = prevTasksRef.current;

		for (const task of tasks) {
			const prevStatus = prevTasks.get(task.id);

			if (
				prevStatus &&
				prevStatus !== "inreview" &&
				task.status === "inreview"
			) {
				showNotification(
					"Task Complete",
					`"${task.title}" is ready for review`,
				);
			}
		}

		const newPrevTasks = new Map<string, string>();
		for (const task of tasks) {
			newPrevTasks.set(task.id, task.status);
		}
		prevTasksRef.current = newPrevTasks;
	}, [tasks, showNotification, permissionGranted]);
};
