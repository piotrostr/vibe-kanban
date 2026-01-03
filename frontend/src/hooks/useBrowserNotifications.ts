import { useEffect, useRef, useCallback } from "react";
import type { TaskWithAttemptStatus } from "shared/types";

/**
 * Hook to show browser notifications when tasks complete (status changes to 'inreview')
 * Automatically requests permission and enables notifications
 * Clicking notification navigates to the task
 */
export const useBrowserNotifications = (
	tasks: TaskWithAttemptStatus[],
	projectId: string | undefined,
) => {
	const prevTasksRef = useRef<Map<string, string>>(new Map());
	const permissionGranted = useRef(false);

	// Auto-request notification permission on mount
	useEffect(() => {
		if (!("Notification" in window)) return;

		if (Notification.permission === "granted") {
			permissionGranted.current = true;
		} else if (Notification.permission !== "denied") {
			Notification.requestPermission().then((permission) => {
				permissionGranted.current = permission === "granted";
			});
		}
	}, []);

	const showNotification = useCallback(
		(title: string, body: string, taskId: string) => {
			if (!("Notification" in window)) return;
			if (Notification.permission !== "granted") return;
			if (!projectId) return;

			const notification = new Notification(title, {
				body,
				icon: "/vibe.jpeg",
				tag: `task-complete-${taskId}`,
			});

			notification.onclick = () => {
				window.focus();
				// Navigate to the task's latest attempt
				window.location.href = `/projects/${projectId}/tasks/${taskId}/attempts/latest`;
				notification.close();
			};
		},
		[projectId],
	);

	// Watch for task status changes to 'inreview'
	useEffect(() => {
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
					task.id,
				);
			}
		}

		// Update previous state
		const newPrevTasks = new Map<string, string>();
		for (const task of tasks) {
			newPrevTasks.set(task.id, task.status);
		}
		prevTasksRef.current = newPrevTasks;
	}, [tasks, showNotification]);
};
