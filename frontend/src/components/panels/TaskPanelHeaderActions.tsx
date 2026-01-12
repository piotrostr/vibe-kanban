import { Button } from "../ui/button";
import { GitPullRequest, X } from "lucide-react";
import type { TaskWithAttemptStatus } from "shared/types";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";

type Task = TaskWithAttemptStatus;

interface TaskPanelHeaderActionsProps {
	task: Task;
	onClose: () => void;
}

export const TaskPanelHeaderActions = ({
	task,
	onClose,
}: TaskPanelHeaderActionsProps) => {
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
							<GitPullRequest className="h-4 w-4" />
						</Button>
					</TooltipTrigger>
					<TooltipContent>View Pull Request</TooltipContent>
				</Tooltip>
			)}
			<Button variant="icon" aria-label="Close" onClick={onClose}>
				<X size={16} />
			</Button>
		</TooltipProvider>
	);
};
