import { Button } from "../ui/button";
import { X } from "lucide-react";
import type { TaskWithAttemptStatus } from "shared/types";
import { ActionsDropdown } from "../ui/actions-dropdown";
import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { LinearIcon } from "../icons/LinearIcon";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

type Task = TaskWithAttemptStatus;

interface TaskPanelHeaderActionsProps {
	task: Task;
	sharedTask?: SharedTaskRecord;
	onClose: () => void;
}

export const TaskPanelHeaderActions = ({
	task,
	sharedTask,
	onClose,
}: TaskPanelHeaderActionsProps) => {
	return (
		<>
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
			<ActionsDropdown task={task} sharedTask={sharedTask} />
			<Button variant="icon" aria-label="Close" onClick={onClose}>
				<X size={16} />
			</Button>
		</>
	);
};
