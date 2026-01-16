import { useEffect, useRef, useCallback, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification,
	onAction,
} from "@tauri-apps/plugin-notification";
import type { TaskWithAttemptStatus } from "shared/types";

const isTauri = "__TAURI__" in window;

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
	const [permissionGranted, setPermissionGranted] = useState(false);
	const navigate = useNavigate();

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

	// Listen for notification clicks
	useEffect(() => {
		if (!isTauri) return;

		let cleanup: (() => void) | null = null;

		onAction((notification) => {
			const data = notification.extra;
			if (
				data &&
				typeof data === "object" &&
				"url" in data &&
				typeof data.url === "string"
			) {
				navigate(data.url);
			}
		}).then((listener) => {
			cleanup = () => listener.unregister();
		});

		return () => {
			cleanup?.();
		};
	}, [navigate]);

	const showNotification = useCallback(
		async (title: string, body: string, taskId: string) => {
			if (!isTauri || !permissionGranted || !projectId) return;

			const targetUrl = `/projects/${projectId}/tasks/${taskId}/attempts/latest`;

			try {
				sendNotification({
					title,
					body,
					extra: { url: targetUrl },
				});
			} catch (error) {
				console.error("Failed to send notification:", error);
			}
		},
		[projectId, permissionGranted],
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
					task.id,
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
