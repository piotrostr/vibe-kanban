import { useTranslation } from "react-i18next";
import {
	Eye,
	FileDiff,
	ClipboardList,
	X,
	GitPullRequest,
	Link,
} from "lucide-react";
import { Button } from "../ui/button";
import { ToggleGroup, ToggleGroupItem } from "../ui/toggle-group";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import type { LayoutMode } from "../layout/TasksLayout";
import type { TaskWithAttemptStatus, RepoBranchStatus } from "shared/types";
import type { Workspace } from "shared/types";
import { ActionsDropdown } from "../ui/actions-dropdown";
import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { usePlanFromEntries } from "@/hooks/usePlanFromEntries";
import { ViewPlanDialog } from "@/components/dialogs";
import { BindPRDialog } from "@/components/dialogs/tasks/BindPRDialog";
import { cn } from "@/lib/utils";

interface AttemptHeaderActionsProps {
	onClose: () => void;
	mode?: LayoutMode;
	onModeChange?: (mode: LayoutMode) => void;
	task: TaskWithAttemptStatus;
	attempt?: Workspace | null;
	sharedTask?: SharedTaskRecord;
	branchStatus?: RepoBranchStatus[] | null;
}

export const AttemptHeaderActions = ({
	onClose,
	mode,
	onModeChange,
	task,
	attempt,
	sharedTask,
	branchStatus,
}: AttemptHeaderActionsProps) => {
	const { t } = useTranslation("tasks");
	const planMarkdown = usePlanFromEntries();

	const repoId = branchStatus?.[0]?.repo_id;
	const hasPr = !!task.pr_url;

	const handlePrClick = () => {
		if (hasPr) {
			window.open(task.pr_url!, "_blank", "noopener,noreferrer");
		} else if (repoId && attempt?.id) {
			BindPRDialog.show({
				attemptId: attempt.id,
				repoId,
			});
		}
	};

	return (
		<>
			{/* PR button - show if PR exists or can bind */}
			{(hasPr || (repoId && attempt?.id)) && (
				<TooltipProvider>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="icon"
								aria-label={
									hasPr
										? t("attemptHeaderActions.viewPR")
										: t("attemptHeaderActions.bindPR")
								}
								onClick={handlePrClick}
							>
								{hasPr ? (
									<GitPullRequest
										className={cn(
											"h-4 w-4",
											task.pr_status === "merged" && "text-purple-500",
										)}
									/>
								) : (
									<Link className="h-4 w-4" />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent side="bottom">
							{hasPr
								? t("attemptHeaderActions.viewPR")
								: t("attemptHeaderActions.bindPR")}
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			)}
			{planMarkdown && (
				<TooltipProvider>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="icon"
								aria-label={t("attemptHeaderActions.viewPlan")}
								onClick={() => ViewPlanDialog.show({ planMarkdown })}
							>
								<ClipboardList className="h-4 w-4" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="bottom">
							{t("attemptHeaderActions.viewPlan")}
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			)}
			{planMarkdown && typeof mode !== "undefined" && onModeChange && (
				<div className="h-4 w-px bg-border" />
			)}
			{typeof mode !== "undefined" && onModeChange && (
				<TooltipProvider>
					<ToggleGroup
						type="single"
						value={mode ?? ""}
						onValueChange={(v) => {
							const newMode = (v as LayoutMode) || null;
							onModeChange(newMode);
						}}
						className="inline-flex gap-4"
						aria-label="Layout mode"
					>
						<Tooltip>
							<TooltipTrigger asChild>
								<ToggleGroupItem
									value="preview"
									aria-label="Preview"
									active={mode === "preview"}
								>
									<Eye className="h-4 w-4" />
								</ToggleGroupItem>
							</TooltipTrigger>
							<TooltipContent side="bottom">
								{t("attemptHeaderActions.preview")}
							</TooltipContent>
						</Tooltip>

						<Tooltip>
							<TooltipTrigger asChild>
								<ToggleGroupItem
									value="diffs"
									aria-label="Diffs"
									active={mode === "diffs"}
								>
									<FileDiff className="h-4 w-4" />
								</ToggleGroupItem>
							</TooltipTrigger>
							<TooltipContent side="bottom">
								{t("attemptHeaderActions.diffs")}
							</TooltipContent>
						</Tooltip>
					</ToggleGroup>
				</TooltipProvider>
			)}
			{typeof mode !== "undefined" && onModeChange && (
				<div className="h-4 w-px bg-border" />
			)}
			<ActionsDropdown task={task} attempt={attempt} sharedTask={sharedTask} />
			<Button variant="icon" aria-label="Close" onClick={onClose}>
				<X size={16} />
			</Button>
		</>
	);
};
