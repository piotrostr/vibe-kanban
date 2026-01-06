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
import { PlusIcon } from "lucide-react";
import { CreateAttemptDialog } from "@/components/dialogs/tasks/CreateAttemptDialog";
import WYSIWYGEditor from "@/components/ui/wysiwyg";
import { DataTable, type ColumnDef } from "@/components/ui/table";
import { useState, useEffect, useCallback, useRef } from "react";

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

	const [title, setTitle] = useState(task?.title || "");
	const [description, setDescription] = useState(task?.description || "");
	const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	// Sync state when task changes
	useEffect(() => {
		setTitle(task?.title || "");
		setDescription(task?.description || "");
	}, [task?.id, task?.title, task?.description]);

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
		</>
	);
};

export default TaskPanel;
