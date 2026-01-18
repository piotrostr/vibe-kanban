import { useTranslation } from "react-i18next";
import {
	FileDiff,
	ClipboardList,
	X,
	GitPullRequest,
	Link,
	CircleDot,
	Check,
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
import type {
	TaskWithAttemptStatus,
	RepoBranchStatus,
	ChecksStatus,
} from "shared/types";
import type { Workspace } from "shared/types";
import { ActionsDropdown } from "../ui/actions-dropdown";
import { usePlanFromEntries } from "@/hooks/usePlanFromEntries";
import { ViewPlanDialog } from "@/components/dialogs";
import { BindPRDialog } from "@/components/dialogs/tasks/BindPRDialog";
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

interface AttemptHeaderActionsProps {
	onClose: () => void;
	mode?: LayoutMode;
	onModeChange?: (mode: LayoutMode) => void;
	task: TaskWithAttemptStatus;
	attempt?: Workspace | null;
	branchStatus?: RepoBranchStatus[] | null;
}

export const AttemptHeaderActions = ({
	onClose,
	mode,
	onModeChange,
	task,
	attempt,
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

	const prTooltip = hasPr
		? `${t("attemptHeaderActions.viewPR")}${task.pr_is_draft ? " (Draft)" : ""}${task.pr_status === "merged" ? " (Merged)" : ""}${task.pr_has_conflicts ? " (Has Conflicts)" : ""}`
		: t("attemptHeaderActions.bindPR");

	return (
		<>
			{/* PR button - show if PR exists or can bind */}
			{(hasPr || (repoId && attempt?.id)) && (
				<TooltipProvider>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="icon"
								aria-label={prTooltip}
								onClick={handlePrClick}
								className="relative"
							>
								{hasPr ? (
									<>
										<GitPullRequest
											className={cn(
												"h-4 w-4",
												task.pr_is_draft && "text-muted-foreground",
												task.pr_status === "merged" && "text-purple-500",
												task.pr_has_conflicts && "text-orange-500",
											)}
										/>
										{task.pr_status === "open" && (
											<span className="absolute -bottom-0.5 -right-0.5">
												{getChecksIcon(task.pr_checks_status)}
											</span>
										)}
									</>
								) : (
									<Link className="h-4 w-4" />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent side="bottom">{prTooltip}</TooltipContent>
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
			<ActionsDropdown task={task} attempt={attempt} />
			<Button variant="icon" aria-label="Close" onClick={onClose}>
				<X size={16} />
			</Button>
		</>
	);
};
