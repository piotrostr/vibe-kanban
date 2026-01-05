import { useCallback, useEffect, useRef, useState } from "react";
import { KanbanCard } from "@/components/ui/shadcn-io/kanban";
import {
	Check,
	CircleDot,
	GitPullRequest,
	Link,
	Loader2,
	X,
	XCircle,
} from "lucide-react";
import { LinearIcon } from "@/components/icons/LinearIcon";
import type {
	ChecksStatus,
	ReviewDecision,
	TaskWithAttemptStatus,
} from "shared/types";
import { ActionsDropdown } from "@/components/ui/actions-dropdown";
import { Button } from "@/components/ui/button";
import { useNavigateWithSearch } from "@/hooks";
import { paths } from "@/lib/paths";
import { attemptsApi } from "@/lib/api";
import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { TaskCardHeader } from "./TaskCardHeader";
import { useTranslation } from "react-i18next";
import { useAuth } from "@/hooks";
import { cn } from "@/lib/utils";

function getChecksIcon(status: ChecksStatus | null | undefined) {
	if (!status || status === "pending") {
		return (
			<CircleDot
				className="h-2.5 w-2.5 text-yellow-500"
				aria-label="Checks pending"
			/>
		);
	}
	if (status === "success") {
		return (
			<Check
				className="h-2.5 w-2.5 text-green-500"
				aria-label="Checks passed"
			/>
		);
	}
	return <X className="h-2.5 w-2.5 text-red-500" aria-label="Checks failed" />;
}

function getReviewBadge(decision: ReviewDecision | null | undefined) {
	if (!decision || decision === "pending") return null;

	const styles: Record<string, string> = {
		approved: "bg-green-500/20 text-green-600 dark:text-green-400",
		changes_requested: "bg-red-500/20 text-red-600 dark:text-red-400",
		review_required: "bg-yellow-500/20 text-yellow-600 dark:text-yellow-400",
	};

	const labels: Record<string, string> = {
		approved: "Approved",
		changes_requested: "Changes",
		review_required: "Review",
	};

	return (
		<span
			className={cn(
				"text-[10px] font-medium px-1 py-0.5 rounded",
				styles[decision],
			)}
		>
			{labels[decision]}
		</span>
	);
}

type Task = TaskWithAttemptStatus;

interface TaskCardProps {
	task: Task;
	index: number;
	status: string;
	onViewDetails: (task: Task) => void;
	isOpen?: boolean;
	projectId: string;
	sharedTask?: SharedTaskRecord;
	/** Project color for unified "Show All" view */
	projectColor?: string;
	/** Project name for unified "Show All" view */
	projectName?: string;
}

export function TaskCard({
	task,
	index,
	status,
	onViewDetails,
	isOpen,
	projectId,
	sharedTask,
	projectColor,
	projectName,
}: TaskCardProps) {
	const { t } = useTranslation("tasks");
	const navigate = useNavigateWithSearch();
	const [isNavigatingToParent, setIsNavigatingToParent] = useState(false);
	const { isSignedIn } = useAuth();

	const handleClick = useCallback(() => {
		onViewDetails(task);
	}, [task, onViewDetails]);

	const handleParentClick = useCallback(
		async (e: React.MouseEvent) => {
			e.stopPropagation();
			if (!task.parent_workspace_id || isNavigatingToParent) return;

			setIsNavigatingToParent(true);
			try {
				const parentAttempt = await attemptsApi.get(task.parent_workspace_id);
				navigate(
					paths.attempt(
						projectId,
						parentAttempt.task_id,
						task.parent_workspace_id,
					),
				);
			} catch (error) {
				console.error("Failed to navigate to parent task attempt:", error);
				setIsNavigatingToParent(false);
			}
		},
		[task.parent_workspace_id, projectId, navigate, isNavigatingToParent],
	);

	const localRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (!isOpen || !localRef.current) return;
		const el = localRef.current;
		requestAnimationFrame(() => {
			el.scrollIntoView({
				block: "center",
				inline: "nearest",
				behavior: "smooth",
			});
		});
	}, [isOpen]);

	// Determine left border styling: project color takes precedence, then shared task indicator
	const hasProjectColor = !!projectColor;
	const hasSharedIndicator =
		(sharedTask || task.shared_task_id) && !hasProjectColor;

	return (
		<KanbanCard
			key={task.id}
			id={task.id}
			name={task.title}
			index={index}
			parent={status}
			onClick={handleClick}
			isOpen={isOpen}
			forwardedRef={localRef}
			dragDisabled={(!!sharedTask || !!task.shared_task_id) && !isSignedIn}
			className={
				hasSharedIndicator
					? 'relative overflow-hidden pl-5 before:absolute before:left-0 before:top-0 before:bottom-0 before:w-[3px] before:bg-card-foreground before:content-[""]'
					: hasProjectColor
						? "relative overflow-hidden pl-5"
						: undefined
			}
			style={
				hasProjectColor
					? {
							borderLeft: `3px solid ${projectColor}`,
						}
					: undefined
			}
		>
			<div className="flex flex-col gap-2">
				{projectName && (
					<div
						className="flex items-center gap-1.5 text-xs text-muted-foreground"
						title={projectName}
					>
						<span
							className="h-2 w-2 rounded-full flex-shrink-0"
							style={{ backgroundColor: projectColor }}
						/>
						<span className="truncate max-w-[150px]">{projectName}</span>
					</div>
				)}
				<TaskCardHeader
					title={task.title}
					avatar={
						sharedTask
							? {
									firstName: sharedTask.assignee_first_name ?? undefined,
									lastName: sharedTask.assignee_last_name ?? undefined,
									username: sharedTask.assignee_username ?? undefined,
								}
							: undefined
					}
					right={
						<>
							{task.has_in_progress_attempt && (
								<Loader2 className="h-4 w-4 animate-spin text-blue-500" />
							)}
							{task.last_attempt_failed && (
								<XCircle className="h-4 w-4 text-destructive" />
							)}
							{task.parent_workspace_id && (
								<Button
									variant="icon"
									onClick={handleParentClick}
									onPointerDown={(e) => e.stopPropagation()}
									onMouseDown={(e) => e.stopPropagation()}
									disabled={isNavigatingToParent}
									title={t("navigateToParent")}
								>
									<Link className="h-4 w-4" />
								</Button>
							)}
							{task.pr_url && (
								<div className="flex items-center gap-1">
									{getReviewBadge(task.pr_review_decision)}
									<Button
										variant="icon"
										onClick={(e) => {
											e.stopPropagation();
											window.open(
												task.pr_url!,
												"_blank",
												"noopener,noreferrer",
											);
										}}
										onPointerDown={(e) => e.stopPropagation()}
										onMouseDown={(e) => e.stopPropagation()}
										title={`View Pull Request${task.pr_is_draft ? " (Draft)" : ""}`}
										className="relative"
									>
										<GitPullRequest
											className={cn(
												"h-4 w-4",
												task.pr_is_draft && "text-muted-foreground",
											)}
										/>
										<span className="absolute -bottom-0.5 -right-0.5">
											{getChecksIcon(task.pr_checks_status)}
										</span>
									</Button>
								</div>
							)}
							{task.linear_url && (
								<Button
									variant="icon"
									onClick={(e) => {
										e.stopPropagation();
										window.open(
											task.linear_url!,
											"_blank",
											"noopener,noreferrer",
										);
									}}
									onPointerDown={(e) => e.stopPropagation()}
									onMouseDown={(e) => e.stopPropagation()}
									title="View in Linear"
								>
									<LinearIcon className="h-4 w-4" />
								</Button>
							)}
							<ActionsDropdown task={task} sharedTask={sharedTask} />
						</>
					}
				/>
				{task.description && (
					<p className="text-sm text-secondary-foreground break-words">
						{task.description.length > 130
							? `${task.description.substring(0, 130)}...`
							: task.description}
					</p>
				)}
			</div>
		</KanbanCard>
	);
}
