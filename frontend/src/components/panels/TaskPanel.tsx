import { useTranslation } from "react-i18next";
import { useProject } from "@/contexts/ProjectContext";
import { useTaskAttemptsWithSessions } from "@/hooks/useTaskAttempts";
import { useTaskAttemptWithSession } from "@/hooks/useTaskAttempt";
import { useTaskMutations } from "@/hooks/useTaskMutations";
import { useNavigateWithSearch } from "@/hooks";
import { paths } from "@/lib/paths";
import type { TaskWithAttemptStatus } from "shared/types";
import type { WorkspaceWithSession } from "@/types/attempt";
import { NewCardContent } from "../ui/new-card";
import { Button } from "../ui/button";
import { Download, Loader2, PlusIcon, Upload } from "lucide-react";
import { CreateAttemptDialog } from "@/components/dialogs/tasks/CreateAttemptDialog";
import WYSIWYGEditor from "@/components/ui/wysiwyg";
import { DataTable, type ColumnDef } from "@/components/ui/table";
import { useState, useEffect, useCallback, useRef } from "react";
import { LinearIcon } from "../icons/LinearIcon";
import { tasksApi } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";

function extractLinearIssueId(linearUrl: string | null): string | null {
	if (!linearUrl) return null;
	const match = linearUrl.match(/\/issue\/([A-Z]+-\d+)/);
	return match ? match[1] : null;
}

const STATUS_LABELS: Record<string, string> = {
	backlog: "Backlog",
	todo: "Todo",
	inprogress: "In Progress",
	inreview: "In Review",
	done: "Done",
	cancelled: "Cancelled",
};

interface TaskPanelProps {
	task: TaskWithAttemptStatus | null;
	/** Optional callback when an attempt is clicked. If not provided, navigates to attempt page. */
	onAttemptClick?: (attemptId: string) => void;
}

const TaskPanel = ({ task, onAttemptClick }: TaskPanelProps) => {
	const { t } = useTranslation("tasks");
	const navigate = useNavigateWithSearch();
	const { projectId } = useProject();
	const { updateTask } = useTaskMutations(projectId || "");
	const queryClient = useQueryClient();

	const [title, setTitle] = useState(task?.title || "");
	const [description, setDescription] = useState(task?.description || "");
	const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	// Linear sync state
	const [isPushing, setIsPushing] = useState(false);
	const [isPulling, setIsPulling] = useState(false);
	const [pushConfirmOpen, setPushConfirmOpen] = useState(false);
	const [pullConfirmOpen, setPullConfirmOpen] = useState(false);
	const [linearState, setLinearState] = useState<{
		title: string;
		status: string;
		statusLabel: string;
	} | null>(null);
	const [linearAssignee, setLinearAssignee] = useState<string | null>(null);

	// Sync state when task changes
	useEffect(() => {
		setTitle(task?.title || "");
		setDescription(task?.description || "");
	}, [task?.id, task?.title, task?.description]);

	// Fetch Linear assignee when task has a linear issue
	useEffect(() => {
		if (!task?.linear_issue_id) {
			setLinearAssignee(null);
			return;
		}

		tasksApi
			.getLinearState(task.id)
			.then((state) => {
				setLinearAssignee(state.issue.assignee?.name ?? null);
			})
			.catch((err) => {
				console.error("Failed to fetch Linear assignee:", err);
				setLinearAssignee(null);
			});
	}, [task?.id, task?.linear_issue_id]);

	const saveChanges = useCallback(
		(newTitle: string, newDescription: string) => {
			if (!task || !projectId) return;

			if (saveTimeoutRef.current) {
				clearTimeout(saveTimeoutRef.current);
			}

			saveTimeoutRef.current = setTimeout(() => {
				updateTask.mutate({
					taskId: task.id,
					data: {
						title: newTitle,
						description: newDescription || null,
						status: task.status,
						parent_workspace_id: task.parent_workspace_id || null,
						image_ids: null,
						sync_to_linear: false,
					},
				});
			}, 500);
		},
		[task, projectId, updateTask],
	);

	const handleTitleChange = useCallback(
		(markdown: string) => {
			// Remove the leading "# " from the markdown heading
			const newTitle = markdown.replace(/^#\s*/, "").trim();
			setTitle(newTitle);
			saveChanges(newTitle, description);
		},
		[description, saveChanges],
	);

	const handleDescriptionChange = useCallback(
		(markdown: string) => {
			setDescription(markdown);
			saveChanges(title, markdown);
		},
		[title, saveChanges],
	);

	// Linear sync handlers
	const handlePushClick = async () => {
		if (!task?.linear_issue_id) return;

		try {
			setIsPushing(true);
			const state = await tasksApi.getLinearState(task.id);
			const linearStatusLabel =
				STATUS_LABELS[state.mapped_status] || state.mapped_status;
			const localStatusLabel = STATUS_LABELS[task.status] || task.status;

			if (state.mapped_status === task.status) {
				await tasksApi.pushToLinear(task.id);
				queryClient.invalidateQueries({ queryKey: ["tasks"] });
			} else {
				setLinearState({
					title: state.issue.title,
					status: state.mapped_status,
					statusLabel: `${linearStatusLabel} -> ${localStatusLabel}`,
				});
				setPushConfirmOpen(true);
			}
		} catch (err) {
			console.error("Failed to check Linear state:", err);
		} finally {
			setIsPushing(false);
		}
	};

	const handlePushConfirm = async () => {
		if (!task) return;
		try {
			setIsPushing(true);
			await tasksApi.pushToLinear(task.id);
			queryClient.invalidateQueries({ queryKey: ["tasks"] });
		} catch (err) {
			console.error("Failed to push to Linear:", err);
		} finally {
			setIsPushing(false);
			setPushConfirmOpen(false);
		}
	};

	const handlePullClick = async () => {
		if (!task?.linear_issue_id) return;

		try {
			setIsPulling(true);
			const state = await tasksApi.getLinearState(task.id);
			const linearStatusLabel =
				STATUS_LABELS[state.mapped_status] || state.mapped_status;
			const localStatusLabel = STATUS_LABELS[task.status] || task.status;

			if (
				state.issue.title === task.title &&
				state.mapped_status === task.status
			) {
				setLinearState(null);
			} else {
				setLinearState({
					title: state.issue.title,
					status: state.mapped_status,
					statusLabel:
						state.mapped_status !== task.status
							? `${localStatusLabel} -> ${linearStatusLabel}`
							: "",
				});
				setPullConfirmOpen(true);
			}
		} catch (err) {
			console.error("Failed to check Linear state:", err);
		} finally {
			setIsPulling(false);
		}
	};

	const handlePullConfirm = async () => {
		if (!task) return;
		try {
			setIsPulling(true);
			await tasksApi.pullFromLinear(task.id);
			queryClient.invalidateQueries({ queryKey: ["tasks"] });
		} catch (err) {
			console.error("Failed to pull from Linear:", err);
		} finally {
			setIsPulling(false);
			setPullConfirmOpen(false);
		}
	};

	const {
		data: attempts = [],
		isLoading: isAttemptsLoading,
		isError: isAttemptsError,
	} = useTaskAttemptsWithSessions(task?.id);

	const { data: parentAttempt, isLoading: isParentLoading } =
		useTaskAttemptWithSession(task?.parent_workspace_id || undefined);

	const formatTimeAgo = (iso: string) => {
		const d = new Date(iso);
		const diffMs = Date.now() - d.getTime();
		const absSec = Math.round(Math.abs(diffMs) / 1000);

		const rtf =
			typeof Intl !== "undefined" &&
			typeof Intl.RelativeTimeFormat === "function"
				? new Intl.RelativeTimeFormat(undefined, { numeric: "auto" })
				: null;

		const to = (value: number, unit: Intl.RelativeTimeFormatUnit) =>
			rtf
				? rtf.format(-value, unit)
				: `${value} ${unit}${value !== 1 ? "s" : ""} ago`;

		if (absSec < 60) return to(Math.round(absSec), "second");
		const mins = Math.round(absSec / 60);
		if (mins < 60) return to(mins, "minute");
		const hours = Math.round(mins / 60);
		if (hours < 24) return to(hours, "hour");
		const days = Math.round(hours / 24);
		if (days < 30) return to(days, "day");
		const months = Math.round(days / 30);
		if (months < 12) return to(months, "month");
		const years = Math.round(months / 12);
		return to(years, "year");
	};

	const displayedAttempts = [...attempts].sort(
		(a, b) =>
			new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
	);

	if (!task) {
		return (
			<div className="text-muted-foreground">
				{t("taskPanel.noTaskSelected")}
			</div>
		);
	}

	const titleContent = `# ${title || "Task"}`;
	const descriptionContent = description || "";

	const attemptColumns: ColumnDef<WorkspaceWithSession>[] = [
		{
			id: "executor",
			header: "",
			accessor: (attempt) => attempt.session?.executor || "Base Agent",
			className: "pr-4",
		},
		{
			id: "branch",
			header: "",
			accessor: (attempt) => attempt.branch || "—",
			className: "pr-4",
		},
		{
			id: "time",
			header: "",
			accessor: (attempt) => formatTimeAgo(attempt.created_at),
			className: "pr-0 text-right",
		},
	];

	return (
		<>
			<NewCardContent>
				<div className="p-6 flex flex-col h-full max-h-[calc(100vh-8rem)]">
					<div className="space-y-3 overflow-y-auto flex-shrink min-h-0">
						<WYSIWYGEditor
							value={titleContent}
							onChange={handleTitleChange}
							projectId={projectId || undefined}
						/>
						<WYSIWYGEditor
							value={descriptionContent}
							onChange={handleDescriptionChange}
							placeholder="Add more details (optional). Type @ to search files."
							projectId={projectId || undefined}
							taskId={task.id}
						/>
					</div>

					<div className="mt-6 flex-shrink-0 space-y-4">
						{task.linear_url && (
							<TooltipProvider>
								<div className="flex items-center gap-2">
									<button
										type="button"
										onClick={() =>
											window.open(
												task.linear_url!,
												"_blank",
												"noopener,noreferrer",
											)
										}
										className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
									>
										<LinearIcon className="h-4 w-4" />
										<span className="font-medium">
											{extractLinearIssueId(task.linear_url)}
										</span>
									</button>
									<Tooltip>
										<TooltipTrigger asChild>
											<Button
												variant="icon"
												size="sm"
												aria-label="Pull from Linear"
												onClick={handlePullClick}
												disabled={isPulling}
											>
												{isPulling ? (
													<Loader2 className="h-3.5 w-3.5 animate-spin" />
												) : (
													<Download className="h-3.5 w-3.5" />
												)}
											</Button>
										</TooltipTrigger>
										<TooltipContent>Pull from Linear</TooltipContent>
									</Tooltip>
									<Tooltip>
										<TooltipTrigger asChild>
											<Button
												variant="icon"
												size="sm"
												aria-label="Push to Linear"
												onClick={handlePushClick}
												disabled={isPushing}
											>
												{isPushing ? (
													<Loader2 className="h-3.5 w-3.5 animate-spin" />
												) : (
													<Upload className="h-3.5 w-3.5" />
												)}
											</Button>
										</TooltipTrigger>
										<TooltipContent>Push to Linear</TooltipContent>
									</Tooltip>
									{linearAssignee && (
										<>
											<div className="flex-1" />
											<span className="text-sm text-muted-foreground">
												{linearAssignee}
											</span>
										</>
									)}
								</div>
							</TooltipProvider>
						)}

						{task.parent_workspace_id && (
							<DataTable
								data={parentAttempt ? [parentAttempt] : []}
								columns={attemptColumns}
								keyExtractor={(attempt) => attempt.id}
								onRowClick={(attempt) => {
									if (onAttemptClick) {
										onAttemptClick(attempt.id);
									} else if (projectId) {
										navigate(
											paths.attempt(projectId, attempt.task_id, attempt.id),
										);
									}
								}}
								isLoading={isParentLoading}
								headerContent="Parent Attempt"
							/>
						)}

						{isAttemptsLoading ? (
							<div className="text-muted-foreground">
								{t("taskPanel.loadingAttempts")}
							</div>
						) : isAttemptsError ? (
							<div className="text-destructive">
								{t("taskPanel.errorLoadingAttempts")}
							</div>
						) : (
							<DataTable
								data={displayedAttempts}
								columns={attemptColumns}
								keyExtractor={(attempt) => attempt.id}
								onRowClick={(attempt) => {
									if (onAttemptClick) {
										onAttemptClick(attempt.id);
									} else if (projectId && task.id) {
										navigate(paths.attempt(projectId, task.id, attempt.id));
									}
								}}
								emptyState={t("taskPanel.noAttempts")}
								headerContent={
									<div className="w-full flex text-left">
										<span className="flex-1">
											{t("taskPanel.attemptsCount", {
												count: displayedAttempts.length,
											})}
										</span>
										<span>
											<Button
												variant="icon"
												onClick={() =>
													CreateAttemptDialog.show({
														taskId: task.id,
														projectId: task.project_id,
													})
												}
											>
												<PlusIcon size={16} />
											</Button>
										</span>
									</div>
								}
							/>
						)}
					</div>
				</div>
			</NewCardContent>

			{/* Push Confirmation Dialog */}
			<Dialog open={pushConfirmOpen} onOpenChange={setPushConfirmOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Push to Linear?</DialogTitle>
						<DialogDescription>
							This will update the Linear issue with local task state.
						</DialogDescription>
					</DialogHeader>
					{linearState?.statusLabel && (
						<p className="text-sm font-medium">
							Status: {linearState.statusLabel}
						</p>
					)}
					<DialogFooter className="gap-2 sm:gap-0">
						<Button variant="outline" onClick={() => setPushConfirmOpen(false)}>
							Cancel
						</Button>
						<Button onClick={handlePushConfirm} disabled={isPushing}>
							{isPushing ? "Pushing..." : "Push to Linear"}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			{/* Pull Confirmation Dialog */}
			<Dialog open={pullConfirmOpen} onOpenChange={setPullConfirmOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Pull from Linear?</DialogTitle>
						<DialogDescription>
							This will update the local task with Linear issue state.
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-2 text-sm">
						{linearState?.title !== task.title && (
							<p className="font-medium">
								Title: &quot;{task.title}&quot; -&gt; &quot;{linearState?.title}
								&quot;
							</p>
						)}
						{linearState?.statusLabel && (
							<p className="font-medium">Status: {linearState.statusLabel}</p>
						)}
					</div>
					<DialogFooter className="gap-2 sm:gap-0">
						<Button variant="outline" onClick={() => setPullConfirmOpen(false)}>
							Cancel
						</Button>
						<Button onClick={handlePullConfirm} disabled={isPulling}>
							{isPulling ? "Pulling..." : "Pull from Linear"}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	);
};

export default TaskPanel;
