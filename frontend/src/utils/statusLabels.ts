import { TaskStatus } from "shared/types";

export const statusLabels: Record<TaskStatus, string> = {
	backlog: "Backlog",
	todo: "To Do",
	inprogress: "In Progress",
	inreview: "In Review",
	done: "Done",
	cancelled: "Cancelled",
};

export const statusBoardColors: Record<TaskStatus, string> = {
	backlog: "--secondary",
	todo: "--neutral-foreground",
	inprogress: "--info",
	inreview: "--warning",
	done: "--success",
	cancelled: "--destructive",
};
