import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Plus, PanelLeftClose, PanelLeft } from "lucide-react";
import { Loader } from "@/components/ui/loader";
import { tasksApi, projectsApi } from "@/lib/api";
import { openTaskForm } from "@/lib/openTaskForm";
import { useProjects } from "@/hooks/useProjects";
import { useAllProjectTasks } from "@/hooks/useAllProjectTasks";
import { LinearSyncConfirmDialog } from "@/components/dialogs/tasks/LinearSyncConfirmDialog";
import TaskPanel from "@/components/panels/TaskPanel";
import { TaskPanelHeaderActions } from "@/components/panels/TaskPanelHeaderActions";
import { NewCard, NewCardHeader } from "@/components/ui/new-card";
import { ProjectProvider } from "@/contexts/ProjectContext";

import TaskKanbanBoard, {
	type KanbanColumnItem,
} from "@/components/tasks/TaskKanbanBoard";
import type { DragEndEvent } from "@/components/ui/shadcn-io/kanban";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";

import type { TaskWithAttemptStatus, TaskStatus } from "shared/types";
import { cn } from "@/lib/utils";

type Task = TaskWithAttemptStatus;

const TASK_STATUSES = [
	"backlog",
	"todo",
	"inprogress",
	"inreview",
	"done",
	"cancelled",
] as const;

const LOCAL_STORAGE_KEY = "allProjectTasks.selectedProjectIds";

const normalizeStatus = (status: string): TaskStatus =>
	status.toLowerCase() as TaskStatus;

/**
 * Unified view showing tasks from all projects in a single kanban board.
 * Tasks are filtered by selected projects via a collapsible sidebar.
 */
export function AllProjectTasks() {
	const { t } = useTranslation(["tasks", "common"]);
	const [searchParams, setSearchParams] = useSearchParams();

	const { projects, projectsById, isLoading: projectsLoading } = useProjects();
	const {
		tasks,
		tasksById,
		isLoading: tasksLoading,
		error: streamError,
	} = useAllProjectTasks();

	const isLoading = projectsLoading || tasksLoading;

	// Project filter state - persisted to localStorage
	const [selectedProjectIds, setSelectedProjectIds] = useState<Set<string>>(
		() => {
			try {
				const saved = localStorage.getItem(LOCAL_STORAGE_KEY);
				if (saved) {
					const parsed = JSON.parse(saved);
					if (Array.isArray(parsed)) {
						return new Set(parsed);
					}
				}
			} catch {
				// Ignore parse errors
			}
			return new Set<string>();
		},
	);
	const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
	const [isRefreshingBacklog, setIsRefreshingBacklog] = useState(false);
	const initializedRef = useRef(false);

	// Task selection from URL
	const selectedTaskId = searchParams.get("taskId");
	const selectedTask = selectedTaskId ? tasksById[selectedTaskId] : null;

	// Persist selected projects to localStorage
	useEffect(() => {
		if (initializedRef.current) {
			localStorage.setItem(
				LOCAL_STORAGE_KEY,
				JSON.stringify(Array.from(selectedProjectIds)),
			);
		}
	}, [selectedProjectIds]);

	// Initialize selected projects when projects load (only once)
	useEffect(() => {
		if (projects.length > 0 && !initializedRef.current) {
			initializedRef.current = true;
			// If nothing saved or saved projects don't exist anymore, select all
			if (selectedProjectIds.size === 0) {
				setSelectedProjectIds(new Set(projects.map((p) => p.id)));
			} else {
				// Filter out any saved project IDs that no longer exist
				const validIds = new Set(
					Array.from(selectedProjectIds).filter((id) =>
						projects.some((p) => p.id === id),
					),
				);
				if (validIds.size === 0) {
					setSelectedProjectIds(new Set(projects.map((p) => p.id)));
				} else if (validIds.size !== selectedProjectIds.size) {
					setSelectedProjectIds(validIds);
				}
			}
		}
	}, [projects, selectedProjectIds]);

	const [collapsedColumns, setCollapsedColumns] = useState<Set<TaskStatus>>(
		new Set(),
	);

	const handleToggleColumn = useCallback((status: TaskStatus) => {
		setCollapsedColumns((prev) => {
			const next = new Set(prev);
			if (next.has(status)) {
				next.delete(status);
			} else {
				next.add(status);
			}
			return next;
		});
	}, []);

	const handleToggleProject = useCallback((projectId: string) => {
		setSelectedProjectIds((prev) => {
			const next = new Set(prev);
			if (next.has(projectId)) {
				next.delete(projectId);
			} else {
				next.add(projectId);
			}
			return next;
		});
	}, []);

	const handleSelectAll = useCallback(() => {
		setSelectedProjectIds(new Set(projects.map((p) => p.id)));
	}, [projects]);

	const handleSelectNone = useCallback(() => {
		setSelectedProjectIds(new Set());
	}, []);

	// Get selected projects as array for task creation
	const selectedProjects = useMemo(() => {
		return projects.filter((p) => selectedProjectIds.has(p.id));
	}, [projects, selectedProjectIds]);

	// Single project selected - used for backlog refresh and default task creation
	const singleSelectedProject =
		selectedProjects.length === 1 ? selectedProjects[0] : null;

	const handleCreateTask = useCallback(() => {
		if (singleSelectedProject) {
			// Single project selected - use it directly
			openTaskForm({ mode: "create", projectId: singleSelectedProject.id });
		} else if (selectedProjects.length > 0) {
			// Multiple projects selected - show selector with only selected projects
			openTaskForm({ mode: "create", projects: selectedProjects });
		} else if (projects.length > 0) {
			// No projects selected - show all projects in selector
			openTaskForm({ mode: "create", projects });
		}
	}, [singleSelectedProject, selectedProjects, projects]);

	const handleRefreshBacklog = useCallback(async () => {
		if (!singleSelectedProject || isRefreshingBacklog) return;
		setIsRefreshingBacklog(true);
		try {
			await projectsApi.syncLinearBacklog(singleSelectedProject.id);
		} catch (err) {
			console.error("Failed to sync Linear backlog:", err);
		} finally {
			setIsRefreshingBacklog(false);
		}
	}, [singleSelectedProject, isRefreshingBacklog]);

	// Filter tasks by selected projects
	const filteredTasks = useMemo(() => {
		if (selectedProjectIds.size === 0) return [];
		return tasks.filter((task) => selectedProjectIds.has(task.project_id));
	}, [tasks, selectedProjectIds]);

	const kanbanColumns = useMemo(() => {
		const columns: Record<TaskStatus, KanbanColumnItem[]> = {
			backlog: [],
			todo: [],
			inprogress: [],
			inreview: [],
			done: [],
			cancelled: [],
		};

		filteredTasks.forEach((task) => {
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
	}, [filteredTasks]);

	const handleViewTaskDetails = useCallback(
		(task: Task) => {
			setSearchParams({ taskId: task.id });
		},
		[setSearchParams],
	);

	const handleCloseTaskPanel = useCallback(() => {
		setSearchParams({});
	}, [setSearchParams]);

	const handleDragEnd = useCallback(
		async (event: DragEndEvent) => {
			const { active, over } = event;
			if (!over || !active.data.current) return;

			const draggedTaskId = active.id as string;
			const newStatus = over.id as Task["status"];
			const task = tasksById[draggedTaskId];
			if (!task || task.status === newStatus) return;

			let syncToLinear = false;

			// Show confirmation dialog for tasks linked to Linear
			if (task.linear_issue_id) {
				const result = await LinearSyncConfirmDialog.show({
					taskTitle: task.title,
					fromStatus: task.status,
					toStatus: newStatus,
				});

				if (result === "cancelled") {
					return;
				}
				syncToLinear = result === "sync";
			}

			try {
				await tasksApi.update(draggedTaskId, {
					title: task.title,
					description: task.description,
					status: newStatus,
					parent_workspace_id: task.parent_workspace_id,
					image_ids: null,
					sync_to_linear: syncToLinear,
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
		filteredTasks.length === 0 ? (
			<div className="max-w-7xl mx-auto mt-8">
				<Card>
					<CardContent className="text-center py-8">
						<p className="text-muted-foreground">
							{selectedProjectIds.size === 0
								? "Select at least one project to view tasks"
								: t("empty.noTasks")}
						</p>
						{selectedProjectIds.size > 0 && projects.length > 0 && (
							<Button className="mt-4" onClick={handleCreateTask}>
								<Plus className="h-4 w-4 mr-2" />
								{t("empty.createFirst")}
							</Button>
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
					selectedTaskId={selectedTask?.id}
					onCreateTask={handleCreateTask}
					projectId={singleSelectedProject?.id ?? ""}
					projectsById={projectsById}
					collapsedColumns={collapsedColumns}
					onToggleColumn={handleToggleColumn}
					onRefreshBacklog={
						singleSelectedProject ? handleRefreshBacklog : undefined
					}
					isRefreshingBacklog={isRefreshingBacklog}
				/>
			</div>
		);

	const taskPanelContent = selectedTask ? (
		<ProjectProvider projectId={selectedTask.project_id}>
			<NewCard className="h-full min-h-0 flex flex-col border-l rounded-none">
				<NewCardHeader
					className="shrink-0"
					actions={
						<TaskPanelHeaderActions
							task={selectedTask}
							sharedTask={undefined}
							onClose={handleCloseTaskPanel}
						/>
					}
				>
					<div className="truncate font-medium">{selectedTask.title}</div>
				</NewCardHeader>
				<TaskPanel task={selectedTask} />
			</NewCard>
		</ProjectProvider>
	) : null;

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

			<div className="flex-1 min-h-0 flex">
				{/* Project filter sidebar - always visible */}
				<div
					className={cn(
						"border-r bg-background transition-all duration-200 flex flex-col shrink-0",
						sidebarCollapsed ? "w-10" : "w-56",
					)}
				>
					<div className="flex items-center justify-between p-2 border-b">
						{!sidebarCollapsed && (
							<span className="text-sm font-medium px-1">Projects</span>
						)}
						<Button
							variant="ghost"
							size="icon"
							className="h-6 w-6"
							onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
						>
							{sidebarCollapsed ? (
								<PanelLeft className="h-4 w-4" />
							) : (
								<PanelLeftClose className="h-4 w-4" />
							)}
						</Button>
					</div>

					{!sidebarCollapsed && (
						<>
							<div className="flex gap-1 p-2 border-b">
								<Button
									variant="ghost"
									size="sm"
									className="h-6 px-2 text-xs"
									onClick={handleSelectAll}
								>
									All
								</Button>
								<Button
									variant="ghost"
									size="sm"
									className="h-6 px-2 text-xs"
									onClick={handleSelectNone}
								>
									None
								</Button>
							</div>

							<div className="flex-1 overflow-y-auto p-2 space-y-1">
								{projects.map((project) => (
									<label
										key={project.id}
										className="flex items-center gap-2 px-1 py-1.5 rounded hover:bg-muted cursor-pointer"
									>
										<Checkbox
											checked={selectedProjectIds.has(project.id)}
											onCheckedChange={() => handleToggleProject(project.id)}
										/>
										<span className="text-sm truncate">{project.name}</span>
									</label>
								))}
							</div>

							{singleSelectedProject && (
								<div className="p-2 border-t text-xs text-muted-foreground">
									Linear sync enabled
								</div>
							)}
						</>
					)}
				</div>

				{/* Main content area with optional task panel */}
				<div className="flex-1 min-h-0 flex">
					{/* Kanban board */}
					<div
						className={cn(
							"min-h-0 p-4 transition-all duration-200",
							selectedTask ? "flex-1" : "flex-1",
						)}
					>
						{kanbanContent}
					</div>

					{/* Task panel */}
					{taskPanelContent && (
						<div className="w-[500px] min-h-0 shrink-0">{taskPanelContent}</div>
					)}
				</div>
			</div>
		</div>
	);
}
