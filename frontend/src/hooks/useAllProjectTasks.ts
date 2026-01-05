import { useCallback, useMemo } from "react";
import { useJsonPatchWsStream } from "./useJsonPatchWsStream";
import type { TaskStatus, TaskWithAttemptStatus } from "shared/types";

type TasksState = {
	tasks: Record<string, TaskWithAttemptStatus>;
};

export interface UseAllProjectTasksResult {
	tasks: TaskWithAttemptStatus[];
	tasksById: Record<string, TaskWithAttemptStatus>;
	tasksByStatus: Record<TaskStatus, TaskWithAttemptStatus[]>;
	isLoading: boolean;
	isConnected: boolean;
	error: string | null;
}

/**
 * Stream all tasks across all projects via WebSocket (JSON Patch).
 * Used for the unified "Show All Projects" view.
 * Server sends initial snapshot: replace /tasks with an object keyed by id.
 * Live updates arrive at /tasks/<id> via add/replace/remove operations.
 */
export const useAllProjectTasks = (): UseAllProjectTasksResult => {
	const endpoint = "/api/tasks/all/stream/ws";

	const initialData = useCallback((): TasksState => ({ tasks: {} }), []);

	const { data, isConnected, error } = useJsonPatchWsStream(
		endpoint,
		true, // always enabled
		initialData,
	);

	const tasksById = useMemo(() => data?.tasks ?? {}, [data?.tasks]);

	const { tasks, tasksByStatus } = useMemo(() => {
		const byStatus: Record<TaskStatus, TaskWithAttemptStatus[]> = {
			backlog: [],
			todo: [],
			inprogress: [],
			inreview: [],
			done: [],
			cancelled: [],
		};

		Object.values(tasksById).forEach((task) => {
			byStatus[task.status]?.push(task);
		});

		// Sort each status group by created_at descending
		(Object.values(byStatus) as TaskWithAttemptStatus[][]).forEach((list) => {
			list.sort(
				(a, b) =>
					new Date(b.created_at as string).getTime() -
					new Date(a.created_at as string).getTime(),
			);
		});

		const sorted = Object.values(tasksById).sort(
			(a, b) =>
				new Date(b.created_at as string).getTime() -
				new Date(a.created_at as string).getTime(),
		);

		return { tasks: sorted, tasksByStatus: byStatus };
	}, [tasksById]);

	return {
		tasks,
		tasksById,
		tasksByStatus,
		isLoading: !data && !error,
		isConnected,
		error,
	};
};
