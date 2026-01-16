import { useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
	onAction,
} from "@tauri-apps/plugin-notification";
import type { TaskWithAttemptStatus } from "shared/types";

/**
 * Hook to show native Tauri notifications when tasks complete (status changes to 'inreview')
 * - Always shows notifications, even when app is focused
 * - Clicking focuses existing window and navigates to the task
 */
export const useTauriNotifications = (
	tasks: TaskWithAttemptStatus[],
	projectId: string | undefined,
) => {
	const prevTasksRef = useRef<Map<string, string>>(new Map());
	const permissionGranted = useRef(false);
	const navigate = useNavigate();

	// Request permission on mount
	useEffect(() => {
		(async () => {
			let granted = await isPermissionGranted();
			if (!granted) {
				const permission = await requestPermission();
				granted = permission === "granted";
			}
			permissionGranted.current = granted;
		})();
	}, []);

	// Listen for notification clicks
	useEffect(() => {
		const listenerPromise = onAction((notification) => {
			const data = notification.extra as { url?: string } | undefined;
			if (data?.url) {
				navigate(data.url);
			}
		});

		return () => {
			listenerPromise.then((listener) => listener.unregister());
		};
	}, [navigate]);

	const showNotification = useCallback(
		async (title: string, body: string, taskId: string) => {
			if (!permissionGranted.current || !projectId) return;

			const targetUrl = `/projects/${projectId}/tasks/${taskId}/attempts/latest`;

			await sendNotification({
				title,
				body,
				extra: { url: targetUrl },
			});
		},
		[projectId],
	);

	// Watch for task status changes to 'inreview'
	useEffect(() => {
		if (!permissionGranted.current) return;

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
					task.id,
				);
			}
		}

		const newPrevTasks = new Map<string, string>();
		for (const task of tasks) {
			newPrevTasks.set(task.id, task.status);
		}
		prevTasksRef.current = newPrevTasks;
	}, [tasks, showNotification]);
};
