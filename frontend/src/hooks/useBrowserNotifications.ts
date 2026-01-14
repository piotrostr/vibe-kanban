import { useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import type { TaskWithAttemptStatus } from "shared/types";

/**
 * Hook to show browser notifications when tasks complete (status changes to 'inreview')
 * - Always shows notifications, even when app is focused
 * - Notifications persist until clicked/dismissed
 * - Clicking focuses existing PWA window and navigates to the task
 */
export const useBrowserNotifications = (
	tasks: TaskWithAttemptStatus[],
	projectId: string | undefined,
) => {
	const prevTasksRef = useRef<Map<string, string>>(new Map());
	const permissionGranted = useRef(false);
	const navigate = useNavigate();

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

	// Listen for notification click messages from service worker
	useEffect(() => {
		const handleMessage = (event: MessageEvent) => {
			if (event.data?.type === "NOTIFICATION_CLICK" && event.data?.url) {
				navigate(event.data.url);
			}
		};

		navigator.serviceWorker?.addEventListener("message", handleMessage);
		return () => {
			navigator.serviceWorker?.removeEventListener("message", handleMessage);
		};
	}, [navigate]);

	const showNotification = useCallback(
		async (title: string, body: string, taskId: string) => {
			if (!("Notification" in window)) return;
			if (Notification.permission !== "granted") return;
			if (!projectId) return;

			const targetUrl = `/projects/${projectId}/tasks/${taskId}/attempts/latest`;

			// Use service worker to show notification - this allows proper window focusing on click
			const registration = await navigator.serviceWorker?.ready;
			if (registration) {
				await registration.showNotification(title, {
					body,
					icon: "/vibe-192.png",
					tag: `task-complete-${taskId}`,
					requireInteraction: true,
					data: { url: targetUrl },
				});
			}
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
