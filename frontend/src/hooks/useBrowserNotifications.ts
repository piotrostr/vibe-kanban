import { useEffect, useRef, useCallback } from "react";
import type { TaskWithAttemptStatus } from "shared/types";

/**
 * Hook to show browser notifications when tasks complete (status changes to 'inreview')
 */
export const useBrowserNotifications = (
	tasks: TaskWithAttemptStatus[],
	enabled: boolean,
) => {
	const prevTasksRef = useRef<Map<string, string>>(new Map());
	const permissionGranted = useRef(false);

	// Request notification permission on mount if enabled
	useEffect(() => {
		if (!enabled) return;
		if (!("Notification" in window)) return;

		if (Notification.permission === "granted") {
			permissionGranted.current = true;
		} else if (Notification.permission !== "denied") {
			Notification.requestPermission().then((permission) => {
				permissionGranted.current = permission === "granted";
			});
		}
	}, [enabled]);

	const showNotification = useCallback((title: string, body: string) => {
		if (!("Notification" in window)) return;
		if (Notification.permission !== "granted") return;

		const notification = new Notification(title, {
			body,
			icon: "/vibe.jpeg",
			tag: "task-complete",
		});

		notification.onclick = () => {
			window.focus();
			notification.close();
		};
	}, []);

	// Watch for task status changes to 'inreview'
	useEffect(() => {
		if (!enabled) return;
		if (!permissionGranted.current && Notification.permission !== "granted")
			return;

		const prevTasks = prevTasksRef.current;

		for (const task of tasks) {
			const prevStatus = prevTasks.get(task.id);

			// Only notify if status changed TO inreview (not on initial load)
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

		// Update previous state
		const newPrevTasks = new Map<string, string>();
		for (const task of tasks) {
			newPrevTasks.set(task.id, task.status);
		}
		prevTasksRef.current = newPrevTasks;
	}, [tasks, enabled, showNotification]);
};
