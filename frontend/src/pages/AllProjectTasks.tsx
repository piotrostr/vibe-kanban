import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { PanelGroup, Panel, PanelResizeHandle } from "react-resizable-panels";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Plus,
	PanelLeftClose,
	PanelLeft,
	ChevronLeft,
	Loader2,
	GitPullRequest,
} from "lucide-react";
import { Loader } from "@/components/ui/loader";
import { tasksApi, projectsApi, attemptsApi } from "@/lib/api";
import { openTaskForm } from "@/lib/openTaskForm";
import { ImportPRAsTaskDialog } from "@/components/dialogs/tasks/ImportPRAsTaskDialog";
import { useProjects } from "@/hooks/useProjects";
import { useAllProjectTasks } from "@/hooks/useAllProjectTasks";
import { useTaskAttemptWithSession } from "@/hooks/useTaskAttempt";
import { useBranchStatus, useAttemptExecution } from "@/hooks";
import { PreviewPanel } from "@/components/panels/PreviewPanel";
import { DiffsPanel } from "@/components/panels/DiffsPanel";
import type { RepoBranchStatus, Workspace } from "shared/types";
import { LinearSyncConfirmDialog } from "@/components/dialogs/tasks/LinearSyncConfirmDialog";
import TaskPanel from "@/components/panels/TaskPanel";
import TaskAttemptPanel from "@/components/panels/TaskAttemptPanel";
import { TaskPanelHeaderActions } from "@/components/panels/TaskPanelHeaderActions";
import { AttemptHeaderActions } from "@/components/panels/AttemptHeaderActions";
import { NewCard, NewCardHeader } from "@/components/ui/new-card";
import { ProjectProvider } from "@/contexts/ProjectContext";
import {
	GitOperationsProvider,
	useGitOperationsError,
} from "@/contexts/GitOperationsContext";
import { ClickedElementsProvider } from "@/contexts/ClickedElementsProvider";
import { ReviewProvider } from "@/contexts/ReviewProvider";
import { ExecutionProcessesProvider } from "@/contexts/ExecutionProcessesContext";
import { EntriesProvider } from "@/contexts/EntriesContext";
import { useSearch } from "@/contexts/SearchContext";
import TodoPanel from "@/components/tasks/TodoPanel";
import { type LayoutMode } from "@/components/layout/TasksLayout";

import TaskKanbanBoard, {
	type KanbanColumnItem,
} from "@/components/tasks/TaskKanbanBoard";
import type { DragEndEvent } from "@/components/ui/shadcn-io/kanban";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertTriangle } from "lucide-react";

import type { TaskWithAttemptStatus, TaskStatus } from "shared/types";
import { cn } from "@/lib/utils";
import { getProjectColor } from "@/utils/projectColors";

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
const PANEL_SIZE_KEY = "allProjectTasks.panelSize";

const DEFAULT_KANBAN_SIZE = 65;
const DEFAULT_PANEL_SIZE = 35;
const MIN_PANEL_SIZE = 20;

const normalizeStatus = (status: string): TaskStatus =>
	status.toLowerCase() as TaskStatus;

function GitErrorBanner() {
	const { error: gitError } = useGitOperationsError();

	if (!gitError) return null;

	return (
		<div className="mx-4 mt-4 p-3 border border-destructive rounded">
			<p className="text-sm text-destructive">{gitError}</p>
		</div>
	);
}

function DiffsPanelContainer({
	attempt,
	selectedTask,
	branchStatus,
}: {
	attempt: Workspace | null;
	selectedTask: TaskWithAttemptStatus | null;
	branchStatus: RepoBranchStatus[] | null;
}) {
	const { isAttemptRunning } = useAttemptExecution(attempt?.id);

	return (
		<DiffsPanel
			key={attempt?.id}
			selectedAttempt={attempt}
			gitOps={
				attempt && selectedTask
					? {
							task: selectedTask,
							branchStatus: branchStatus ?? null,
							isAttemptRunning,
							selectedBranch: branchStatus?.[0]?.target_branch_name ?? null,
						}
					: undefined
			}
		/>
	);
}

/**
 * Unified view showing tasks from all projects in a single kanban board.
 * Tasks are filtered by selected projects via a collapsible sidebar.
 */
export function AllProjectTasks() {
	const { t } = useTranslation(["tasks", "common"]);
	const [searchParams, setSearchParams] = useSearchParams();
	const { query: searchQuery } = useSearch();

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
	const [mode, setMode] = useState<LayoutMode>(null);

	// Task and attempt selection from URL
	const selectedTaskId = searchParams.get("taskId");
	const selectedAttemptId = searchParams.get("attemptId");
	const selectedTask = selectedTaskId ? tasksById[selectedTaskId] : null;

	// Fetch attempt data when attemptId is present
	const { data: attempt, isLoading: isAttemptLoading } =
		useTaskAttemptWithSession(selectedAttemptId || undefined);

	// Fetch branch status for diffs panel
	const { data: branchStatus } = useBranchStatus(attempt?.id);

	const isTaskView = selectedTask && !selectedAttemptId;
	const isAttemptView = selectedTask && selectedAttemptId && attempt;

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

	const handleImportFromPR = useCallback(() => {
		// Import from PR only works with a single project selected
		if (singleSelectedProject) {
			ImportPRAsTaskDialog.show({ projectId: singleSelectedProject.id });
		}
	}, [singleSelectedProject]);

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

	// Filter tasks by selected projects and search query
	const filteredTasks = useMemo(() => {
		if (selectedProjectIds.size === 0) return [];
		const lowerQuery = searchQuery.toLowerCase().trim();
		return tasks.filter((task) => {
			if (!selectedProjectIds.has(task.project_id)) return false;
			if (!lowerQuery) return true;
			const titleMatch = task.title.toLowerCase().includes(lowerQuery);
			const descMatch = task.description?.toLowerCase().includes(lowerQuery);
			return titleMatch || descMatch;
		});
	}, [tasks, selectedProjectIds, searchQuery]);

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

	const recentTasks = useMemo(() => {
		return [...tasks]
			.sort(
				(a, b) =>
					new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
			)
			.slice(0, 20);
	}, [tasks]);

	const handleViewTaskDetails = useCallback(
		async (task: Task) => {
			try {
				const attempts = await attemptsApi.getAll(task.id);
				if (attempts.length === 1) {
					// Single attempt - navigate directly to attempt view
					setSearchParams({ taskId: task.id, attemptId: attempts[0].id });
				} else {
					// Multiple or no attempts - show task panel
					setSearchParams({ taskId: task.id });
				}
			} catch {
				// On error, fall back to task view
				setSearchParams({ taskId: task.id });
			}
		},
		[setSearchParams],
	);

	const handleAttemptClick = useCallback(
		(attemptId: string) => {
			if (selectedTaskId) {
				setSearchParams({ taskId: selectedTaskId, attemptId });
			}
		},
		[selectedTaskId, setSearchParams],
	);

	const handleBackToTask = useCallback(() => {
		if (selectedTaskId) {
			setSearchParams({ taskId: selectedTaskId });
		}
	}, [selectedTaskId, setSearchParams]);

	const handleClosePanel = useCallback(() => {
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
					onImportFromPR={
						singleSelectedProject ? handleImportFromPR : undefined
					}
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

	// Task panel content (no attempt selected)
	const taskPanelContent =
		selectedTask && isTaskView ? (
			<ProjectProvider projectId={selectedTask.project_id}>
				<NewCard className="h-full min-h-0 flex flex-col border-l rounded-none">
					<NewCardHeader
						className="shrink-0"
						actions={
							<TaskPanelHeaderActions
								task={selectedTask}
								onClose={handleClosePanel}
							/>
						}
					>
						<div className="truncate font-medium">{selectedTask.title}</div>
					</NewCardHeader>
					<TaskPanel task={selectedTask} onAttemptClick={handleAttemptClick} />
				</NewCard>
			</ProjectProvider>
		) : null;

	// Attempt view content - renders both attempt panel and aux content inside providers
	const attemptViewContent =
		selectedTask && isAttemptView ? (
			<ProjectProvider projectId={selectedTask.project_id}>
				<EntriesProvider key={attempt?.id}>
					<GitOperationsProvider attemptId={attempt?.id}>
						<ClickedElementsProvider attempt={attempt}>
							<ReviewProvider attemptId={attempt?.id}>
								<ExecutionProcessesProvider attemptId={attempt?.id}>
									{mode ? (
										// When mode is set, show attempt | aux split
										<PanelGroup direction="horizontal" className="h-full">
											<Panel
												id="attempt-inner"
												order={1}
												defaultSize={40}
												minSize={MIN_PANEL_SIZE}
												className="min-w-0 min-h-0 overflow-hidden"
											>
												<NewCard className="h-full min-h-0 flex flex-col border-l rounded-none">
													<NewCardHeader
														className="shrink-0"
														actions={
															<AttemptHeaderActions
																mode={mode}
																onModeChange={setMode}
																task={selectedTask}
																attempt={attempt ?? null}
																onClose={handleClosePanel}
															/>
														}
													>
														<div className="flex items-center gap-2">
															<Button
																variant="ghost"
																size="icon"
																className="h-6 w-6"
																onClick={handleBackToTask}
															>
																<ChevronLeft className="h-4 w-4" />
															</Button>
															<div className="truncate">
																<span className="font-medium">
																	{attempt?.branch || "Attempt"}
																</span>
																<span className="text-muted-foreground ml-2 text-sm">
																	{selectedTask.title}
																</span>
															</div>
														</div>
													</NewCardHeader>
													<TaskAttemptPanel
														attempt={attempt}
														task={selectedTask}
													>
														{({ logs, followUp }) => (
															<>
																<GitErrorBanner />
																<div className="flex-1 min-h-0 flex flex-col">
																	<div className="flex-1 min-h-0 flex flex-col">
																		{logs}
																	</div>
																	<div className="shrink-0 border-t">
																		<div className="mx-auto w-full max-w-[50rem]">
																			<TodoPanel />
																		</div>
																	</div>
																	<div className="min-h-0 max-h-[50%] border-t overflow-hidden bg-background">
																		<div className="mx-auto w-full max-w-[50rem] h-full min-h-0">
																			{followUp}
																		</div>
																	</div>
																</div>
															</>
														)}
													</TaskAttemptPanel>
												</NewCard>
											</Panel>

											<PanelResizeHandle
												className={cn(
													"relative z-30 w-1 bg-border cursor-col-resize group touch-none",
													"focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
													"hover:bg-primary/20 transition-colors",
												)}
											>
												<div className="pointer-events-none absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex flex-col items-center gap-1 bg-muted/90 border border-border rounded-full px-1.5 py-3 opacity-0 group-hover:opacity-100 transition-opacity shadow-sm">
													<span className="w-1 h-1 rounded-full bg-muted-foreground" />
													<span className="w-1 h-1 rounded-full bg-muted-foreground" />
													<span className="w-1 h-1 rounded-full bg-muted-foreground" />
												</div>
											</PanelResizeHandle>

											<Panel
												id="aux-inner"
												order={2}
												defaultSize={60}
												minSize={MIN_PANEL_SIZE}
												className="min-w-0 min-h-0 overflow-hidden"
											>
												<div className="h-full w-full">
													{mode === "preview" && <PreviewPanel />}
													{mode === "diffs" && (
														<DiffsPanelContainer
															attempt={attempt ?? null}
															selectedTask={selectedTask}
															branchStatus={branchStatus ?? null}
														/>
													)}
												</div>
											</Panel>
										</PanelGroup>
									) : (
										// No mode - just show attempt panel
										<NewCard className="h-full min-h-0 flex flex-col border-l rounded-none">
											<NewCardHeader
												className="shrink-0"
												actions={
													<AttemptHeaderActions
														mode={mode}
														onModeChange={setMode}
														task={selectedTask}
														attempt={attempt ?? null}
														onClose={handleClosePanel}
													/>
												}
											>
												<div className="flex items-center gap-2">
													<Button
														variant="ghost"
														size="icon"
														className="h-6 w-6"
														onClick={handleBackToTask}
													>
														<ChevronLeft className="h-4 w-4" />
													</Button>
													<div className="truncate">
														<span className="font-medium">
															{attempt?.branch || "Attempt"}
														</span>
														<span className="text-muted-foreground ml-2 text-sm">
															{selectedTask.title}
														</span>
													</div>
												</div>
											</NewCardHeader>
											<TaskAttemptPanel attempt={attempt} task={selectedTask}>
												{({ logs, followUp }) => (
													<>
														<GitErrorBanner />
														<div className="flex-1 min-h-0 flex flex-col">
															<div className="flex-1 min-h-0 flex flex-col">
																{logs}
															</div>
															<div className="shrink-0 border-t">
																<div className="mx-auto w-full max-w-[50rem]">
																	<TodoPanel />
																</div>
															</div>
															<div className="min-h-0 max-h-[50%] border-t overflow-hidden bg-background">
																<div className="mx-auto w-full max-w-[50rem] h-full min-h-0">
																	{followUp}
																</div>
															</div>
														</div>
													</>
												)}
											</TaskAttemptPanel>
										</NewCard>
									)}
								</ExecutionProcessesProvider>
							</ReviewProvider>
						</ClickedElementsProvider>
					</GitOperationsProvider>
				</EntriesProvider>
			</ProjectProvider>
		) : null;

	const panelContent =
		attemptViewContent ||
		taskPanelContent ||
		(isAttemptLoading && selectedAttemptId ? (
			<NewCard className="h-full min-h-0 flex flex-col border-l rounded-none">
				<div className="flex items-center justify-center h-full">
					<Loader message="Loading attempt..." size={24} />
				</div>
			</NewCard>
		) : null);

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
										<span
											className="h-2 w-2 rounded-full flex-shrink-0"
											style={{ backgroundColor: getProjectColor(project.id) }}
										/>
										<span className="text-sm truncate">{project.name}</span>
									</label>
								))}

								{recentTasks.length > 0 && (
									<div className="border-t mt-2 pt-2">
										<div className="px-1 pb-1 text-xs font-medium text-muted-foreground">
											Recent
										</div>
										{recentTasks.map((task) => {
											const projectColor = getProjectColor(task.project_id);
											return (
												<button
													key={task.id}
													type="button"
													onClick={() => handleViewTaskDetails(task)}
													className="w-full text-left px-1 py-1.5 text-sm rounded hover:bg-muted flex items-center gap-1.5"
												>
													<span
														className="h-2 w-2 rounded-full flex-shrink-0"
														style={{ backgroundColor: projectColor }}
													/>
													<span className="truncate flex-1 min-w-0">
														{task.title}
													</span>
													{task.has_in_progress_attempt && (
														<Loader2 className="h-3 w-3 animate-spin text-blue-500 flex-shrink-0" />
													)}
													{task.pr_url && !task.has_in_progress_attempt && (
														<GitPullRequest
															className={cn(
																"h-3 w-3 flex-shrink-0",
																task.pr_status === "merged"
																	? "text-purple-500"
																	: "text-muted-foreground",
															)}
														/>
													)}
												</button>
											);
										})}
									</div>
								)}
							</div>

							{singleSelectedProject && (
								<div className="p-2 border-t text-xs text-muted-foreground">
									Linear sync enabled
								</div>
							)}
						</>
					)}
				</div>

				{/* Main content area with optional panel */}
				<div className="flex-1 min-w-0 min-h-0 overflow-hidden">
					{attemptViewContent && mode ? (
						// When mode is set, attemptViewContent handles its own layout (attempt | aux)
						// Hide kanban and show full-width attemptViewContent
						<div className="h-full">{attemptViewContent}</div>
					) : panelContent ? (
						// When no mode, show kanban | details panel
						<PanelGroup
							direction="horizontal"
							className="h-full"
							onLayout={(sizes) => {
								if (sizes.length === 2) {
									try {
										localStorage.setItem(PANEL_SIZE_KEY, JSON.stringify(sizes));
									} catch {
										// Ignore errors
									}
								}
							}}
						>
							<Panel
								id="kanban"
								order={1}
								defaultSize={(() => {
									try {
										const saved = localStorage.getItem(PANEL_SIZE_KEY);
										if (saved) {
											const parsed = JSON.parse(saved);
											if (Array.isArray(parsed) && parsed.length === 2) {
												return parsed[0];
											}
										}
									} catch {
										// Ignore errors
									}
									return DEFAULT_KANBAN_SIZE;
								})()}
								minSize={MIN_PANEL_SIZE}
								className="min-w-0 min-h-0 overflow-hidden"
							>
								<div className="h-full p-4 overflow-auto">{kanbanContent}</div>
							</Panel>

							<PanelResizeHandle
								className={cn(
									"relative z-30 w-1 bg-border cursor-col-resize group touch-none",
									"focus:outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
									"hover:bg-primary/20 transition-colors",
								)}
							>
								<div className="pointer-events-none absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex flex-col items-center gap-1 bg-muted/90 border border-border rounded-full px-1.5 py-3 opacity-0 group-hover:opacity-100 transition-opacity shadow-sm">
									<span className="w-1 h-1 rounded-full bg-muted-foreground" />
									<span className="w-1 h-1 rounded-full bg-muted-foreground" />
									<span className="w-1 h-1 rounded-full bg-muted-foreground" />
								</div>
							</PanelResizeHandle>

							<Panel
								id="details"
								order={2}
								defaultSize={(() => {
									try {
										const saved = localStorage.getItem(PANEL_SIZE_KEY);
										if (saved) {
											const parsed = JSON.parse(saved);
											if (Array.isArray(parsed) && parsed.length === 2) {
												return parsed[1];
											}
										}
									} catch {
										// Ignore errors
									}
									return DEFAULT_PANEL_SIZE;
								})()}
								minSize={MIN_PANEL_SIZE}
								className="min-w-0 min-h-0 overflow-hidden"
							>
								<div className="h-full overflow-auto">{panelContent}</div>
							</Panel>
						</PanelGroup>
					) : (
						<div className="h-full p-4 overflow-auto">{kanbanContent}</div>
					)}
				</div>
			</div>
		</div>
	);
}
