import { useState } from "react";
import { Button } from "../ui/button";
import { X, Upload, Download, Loader2 } from "lucide-react";
import type { TaskWithAttemptStatus } from "shared/types";
import { ActionsDropdown } from "../ui/actions-dropdown";
import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { GitHubIcon } from "../icons/GitHubIcon";
import { LinearIcon } from "../icons/LinearIcon";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import { tasksApi } from "@/lib/api";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "../ui/dialog";
import { useQueryClient } from "@tanstack/react-query";

type Task = TaskWithAttemptStatus;

interface TaskPanelHeaderActionsProps {
	task: Task;
	sharedTask?: SharedTaskRecord;
	onClose: () => void;
}

const STATUS_LABELS: Record<string, string> = {
	backlog: "Backlog",
	todo: "Todo",
	inprogress: "In Progress",
	inreview: "In Review",
	done: "Done",
	cancelled: "Cancelled",
};

export const TaskPanelHeaderActions = ({
	task,
	sharedTask,
	onClose,
}: TaskPanelHeaderActionsProps) => {
	const queryClient = useQueryClient();
	const [isPushing, setIsPushing] = useState(false);
	const [isPulling, setIsPulling] = useState(false);
	const [pushConfirmOpen, setPushConfirmOpen] = useState(false);
	const [pullConfirmOpen, setPullConfirmOpen] = useState(false);
	const [linearState, setLinearState] = useState<{
		title: string;
		status: string;
		statusLabel: string;
	} | null>(null);

	const handlePushClick = async () => {
		if (!task.linear_issue_id) return;

		try {
			setIsPushing(true);
			// Fetch current Linear state to show what will change
			const state = await tasksApi.getLinearState(task.id);
			const linearStatusLabel =
				STATUS_LABELS[state.mapped_status] || state.mapped_status;
			const localStatusLabel = STATUS_LABELS[task.status] || task.status;

			// Check if there are differences
			if (state.mapped_status === task.status) {
				// No status difference, just push (might update title/description)
				await tasksApi.pushToLinear(task.id);
				queryClient.invalidateQueries({ queryKey: ["tasks"] });
			} else {
				// Show confirmation with status change
				setLinearState({
					title: state.issue.title,
					status: state.mapped_status,
					statusLabel: `${linearStatusLabel} → ${localStatusLabel}`,
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
		if (!task.linear_issue_id) return;

		try {
			setIsPulling(true);
			// Fetch current Linear state to show what will change
			const state = await tasksApi.getLinearState(task.id);
			const linearStatusLabel =
				STATUS_LABELS[state.mapped_status] || state.mapped_status;
			const localStatusLabel = STATUS_LABELS[task.status] || task.status;

			// Check if there are differences
			if (
				state.issue.title === task.title &&
				state.mapped_status === task.status
			) {
				// No difference, nothing to pull
				setLinearState(null);
			} else {
				// Show confirmation with what will change
				setLinearState({
					title: state.issue.title,
					status: state.mapped_status,
					statusLabel:
						state.mapped_status !== task.status
							? `${localStatusLabel} → ${linearStatusLabel}`
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

	const hasLinear = !!task.linear_issue_id;

	return (
		<TooltipProvider>
			{task.pr_url && (
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							variant="icon"
							aria-label="View Pull Request"
							onClick={() => window.open(task.pr_url!, "_blank")}
						>
							<GitHubIcon className="h-4 w-4" />
						</Button>
					</TooltipTrigger>
					<TooltipContent>View Pull Request</TooltipContent>
				</Tooltip>
			)}
			{task.linear_url && (
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							variant="icon"
							aria-label="Open in Linear"
							onClick={() => window.open(task.linear_url!, "_blank")}
						>
							<LinearIcon className="h-4 w-4" />
						</Button>
					</TooltipTrigger>
					<TooltipContent>Open in Linear</TooltipContent>
				</Tooltip>
			)}
			{hasLinear && (
				<>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="icon"
								aria-label="Pull from Linear"
								onClick={handlePullClick}
								disabled={isPulling}
							>
								{isPulling ? (
									<Loader2 className="h-4 w-4 animate-spin" />
								) : (
									<Download className="h-4 w-4" />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent>Pull from Linear</TooltipContent>
					</Tooltip>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="icon"
								aria-label="Push to Linear"
								onClick={handlePushClick}
								disabled={isPushing}
							>
								{isPushing ? (
									<Loader2 className="h-4 w-4 animate-spin" />
								) : (
									<Upload className="h-4 w-4" />
								)}
							</Button>
						</TooltipTrigger>
						<TooltipContent>Push to Linear</TooltipContent>
					</Tooltip>
				</>
			)}
			<ActionsDropdown task={task} sharedTask={sharedTask} />
			<Button variant="icon" aria-label="Close" onClick={onClose}>
				<X size={16} />
			</Button>

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
								Title: &quot;{task.title}&quot; → &quot;{linearState?.title}
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
		</TooltipProvider>
	);
};
