import { useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Plus } from "lucide-react";
import { Loader } from "@/components/ui/loader";
import { tasksApi } from "@/lib/api";
import { openTaskForm } from "@/lib/openTaskForm";
import { useProjects } from "@/hooks/useProjects";
import { useAllProjectTasks } from "@/hooks/useAllProjectTasks";

import TaskKanbanBoard, {
	type KanbanColumnItem,
} from "@/components/tasks/TaskKanbanBoard";
import type { DragEndEvent } from "@/components/ui/shadcn-io/kanban";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";

import type { TaskWithAttemptStatus, TaskStatus } from "shared/types";
import { paths } from "@/lib/paths";

type Task = TaskWithAttemptStatus;

const TASK_STATUSES = [
	"backlog",
	"todo",
	"inprogress",
	"inreview",
	"done",
	"cancelled",
] as const;

const normalizeStatus = (status: string): TaskStatus =>
	status.toLowerCase() as TaskStatus;

/**
 * Unified view showing tasks from all projects in a single kanban board.
 * Tasks are color-coded by project for visual differentiation.
 */
export function AllProjectTasks() {
	const { t } = useTranslation(["tasks", "common"]);
	const navigate = useNavigate();

	const { projects, projectsById, isLoading: projectsLoading } = useProjects();
	const {
		tasks,
		tasksById,
		isLoading: tasksLoading,
		error: streamError,
	} = useAllProjectTasks();

	const isLoading = projectsLoading || tasksLoading;

	const handleCreateTask = useCallback(() => {
		// For unified view, we need the user to select a project first
		// Open the task form with the first project as default, or show project selector
		if (projects.length > 0) {
			openTaskForm({ mode: "create", projectId: projects[0].id });
		}
	}, [projects]);

	const kanbanColumns = useMemo(() => {
		const columns: Record<TaskStatus, KanbanColumnItem[]> = {
			backlog: [],
			todo: [],
			inprogress: [],
			inreview: [],
			done: [],
			cancelled: [],
		};

		tasks.forEach((task) => {
			const statusKey = normalizeStatus(task.status);
			columns[statusKey].push({
				type: "task",
				task,
			});
		});

		const getTimestamp = (item: KanbanColumnItem) => {
			return new Date(item.task.created_at as string).getTime();
		};

		TASK_STATUSES.forEach((status) => {
			columns[status].sort((a, b) => getTimestamp(b) - getTimestamp(a));
		});

		return columns;
	}, [tasks]);

	const handleViewTaskDetails = useCallback(
		(task: Task) => {
			const projectId = task.project_id;
			navigate(`${paths.task(projectId, task.id)}/attempts/latest`);
		},
		[navigate],
	);

	const handleDragEnd = useCallback(
		async (event: DragEndEvent) => {
			const { active, over } = event;
			if (!over || !active.data.current) return;

			const draggedTaskId = active.id as string;
			const newStatus = over.id as Task["status"];
			const task = tasksById[draggedTaskId];
			if (!task || task.status === newStatus) return;

			try {
				await tasksApi.update(draggedTaskId, {
					title: task.title,
					description: task.description,
					status: newStatus,
					parent_workspace_id: task.parent_workspace_id,
					image_ids: null,
					sync_to_linear: true,
				});
			} catch (err) {
				console.error("Failed to update task status:", err);
			}
		},
		[tasksById],
	);

	if (isLoading && tasks.length === 0) {
		return <Loader message={t("loading")} size={32} className="py-8" />;
	}

	const kanbanContent =
		tasks.length === 0 ? (
			<div className="max-w-7xl mx-auto mt-8">
				<Card>
					<CardContent className="text-center py-8">
						<p className="text-muted-foreground">{t("empty.noTasks")}</p>
						{projects.length > 0 ? (
							<Button className="mt-4" onClick={handleCreateTask}>
								<Plus className="h-4 w-4 mr-2" />
								{t("empty.createFirst")}
							</Button>
						) : (
							<p className="mt-4 text-sm text-muted-foreground">
								Create a project first to add tasks
							</p>
						)}
					</CardContent>
				</Card>
			</div>
		) : (
			<div className="w-full h-full overflow-x-auto overflow-y-auto overscroll-x-contain">
				<TaskKanbanBoard
					columns={kanbanColumns}
					onDragEnd={handleDragEnd}
					onViewTaskDetails={handleViewTaskDetails}
					selectedTaskId={undefined}
					onCreateTask={handleCreateTask}
					projectId=""
					projectsById={projectsById}
				/>
			</div>
		);

	return (
		<div className="min-h-full h-full flex flex-col">
			{streamError && (
				<Alert className="w-full z-30 xl:sticky xl:top-0">
					<AlertTitle className="flex items-center gap-2">
						<AlertTriangle size="16" />
						{t("common:states.reconnecting")}
					</AlertTitle>
					<AlertDescription>{streamError}</AlertDescription>
				</Alert>
			)}

			<div className="flex-1 min-h-0 p-4">{kanbanContent}</div>
		</div>
	);
}
