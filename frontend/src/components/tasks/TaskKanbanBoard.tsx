import { memo } from "react";
import { useAuth } from "@/hooks";
import {
	type DragEndEvent,
	KanbanBoard,
	KanbanBoardCollapsed,
	KanbanCards,
	KanbanHeader,
	KanbanProvider,
} from "@/components/ui/shadcn-io/kanban";
import { TaskCard } from "./TaskCard";
import type { Project, TaskStatus, TaskWithAttemptStatus } from "shared/types";
import { statusBoardColors, statusLabels } from "@/utils/statusLabels";
import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { SharedTaskCard } from "./SharedTaskCard";
import { getProjectColor } from "@/utils/projectColors";

export type KanbanColumnItem =
	| {
			type: "task";
			task: TaskWithAttemptStatus;
			sharedTask?: SharedTaskRecord;
	  }
	| {
			type: "shared";
			task: SharedTaskRecord;
	  };

export type KanbanColumns = Record<TaskStatus, KanbanColumnItem[]>;

interface TaskKanbanBoardProps {
	columns: KanbanColumns;
	onDragEnd: (event: DragEndEvent) => void;
	onViewTaskDetails: (task: TaskWithAttemptStatus) => void;
	onViewSharedTask?: (task: SharedTaskRecord) => void;
	selectedTaskId?: string;
	selectedSharedTaskId?: string | null;
	onCreateTask?: () => void;
	onImportFromPR?: () => void;
	projectId: string;
	onRefreshBacklog?: () => void;
	isRefreshingBacklog?: boolean;
	collapsedColumns?: Set<TaskStatus>;
	onToggleColumn?: (status: TaskStatus) => void;
	/** For unified "Show All" view: map of project IDs to project objects */
	projectsById?: Record<string, Project>;
}

function TaskKanbanBoard({
	columns,
	onDragEnd,
	onViewTaskDetails,
	onViewSharedTask,
	selectedTaskId,
	selectedSharedTaskId,
	onCreateTask,
	onImportFromPR,
	projectId,
	onRefreshBacklog,
	isRefreshingBacklog,
	collapsedColumns,
	onToggleColumn,
	projectsById,
}: TaskKanbanBoardProps) {
	const { userId } = useAuth();

	return (
		<KanbanProvider onDragEnd={onDragEnd}>
			{Object.entries(columns).map(([status, items]) => {
				const statusKey = status as TaskStatus;
				const isBacklog = statusKey === "backlog";
				const isCollapsed = collapsedColumns?.has(statusKey);

				if (isCollapsed) {
					return (
						<KanbanBoardCollapsed
							key={status}
							id={statusKey}
							name={statusLabels[statusKey]}
							color={statusBoardColors[statusKey]}
							count={items.length}
							onExpand={() => onToggleColumn?.(statusKey)}
						/>
					);
				}

				return (
					<KanbanBoard key={status} id={statusKey}>
						<KanbanHeader
							name={statusLabels[statusKey]}
							color={statusBoardColors[statusKey]}
							count={items.length}
							onAddTask={onCreateTask}
							onImportFromPR={onImportFromPR}
							onRefresh={isBacklog ? onRefreshBacklog : undefined}
							isRefreshing={isBacklog ? isRefreshingBacklog : undefined}
							onToggleCollapse={
								onToggleColumn ? () => onToggleColumn(statusKey) : undefined
							}
						/>
						<KanbanCards>
							{items.map((item, index) => {
								const isOwnTask =
									item.type === "task" &&
									(!item.sharedTask?.assignee_user_id ||
										!userId ||
										item.sharedTask?.assignee_user_id === userId);

								if (isOwnTask) {
									// Get project info for unified view
									const taskProjectId = item.task.project_id;
									const project = projectsById?.[taskProjectId];
									const projectColor = project
										? getProjectColor(taskProjectId)
										: undefined;
									const projectName = project?.name;

									return (
										<TaskCard
											key={item.task.id}
											task={item.task}
											index={index}
											status={statusKey}
											onViewDetails={onViewTaskDetails}
											isOpen={selectedTaskId === item.task.id}
											projectId={projectId || taskProjectId}
											sharedTask={item.sharedTask}
											projectColor={projectColor}
											projectName={projectName}
										/>
									);
								}

								const sharedTask =
									item.type === "shared" ? item.task : item.sharedTask!;

								return (
									<SharedTaskCard
										key={`shared-${item.task.id}`}
										task={sharedTask}
										index={index}
										status={statusKey}
										isSelected={selectedSharedTaskId === item.task.id}
										onViewDetails={onViewSharedTask}
									/>
								);
							})}
						</KanbanCards>
					</KanbanBoard>
				);
			})}
		</KanbanProvider>
	);
}

export default memo(TaskKanbanBoard);
