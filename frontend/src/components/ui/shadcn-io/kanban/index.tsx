"use client";

import { Card } from "@/components/ui/card";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { DragEndEvent, Modifier } from "@dnd-kit/core";
import {
	DndContext,
	PointerSensor,
	rectIntersection,
	useDraggable,
	useDroppable,
	useSensor,
	useSensors,
} from "@dnd-kit/core";
import { type ReactNode, type Ref, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import { Plus, RefreshCw, Eye, EyeOff } from "lucide-react";
import type { ClientRect } from "@dnd-kit/core";
import type { Transform } from "@dnd-kit/utilities";
import { Button } from "../../button";
export type { DragEndEvent } from "@dnd-kit/core";

export type Status = {
	id: string;
	name: string;
	color: string;
};

export type Feature = {
	id: string;
	name: string;
	startAt: Date;
	endAt: Date;
	status: Status;
};

export type KanbanBoardProps = {
	id: Status["id"];
	children: ReactNode;
	className?: string;
};

export const KanbanBoard = ({ id, children, className }: KanbanBoardProps) => {
	const { isOver, setNodeRef } = useDroppable({ id });

	return (
		<div
			className={cn(
				"flex min-h-40 flex-col min-w-[200px] max-w-[400px] w-[300px]",
				isOver ? "outline-primary" : "outline-black",
				className,
			)}
			ref={setNodeRef}
		>
			{children}
		</div>
	);
};

export type KanbanCardProps = Pick<Feature, "id" | "name"> & {
	index: number;
	parent: string;
	children?: ReactNode;
	className?: string;
	onClick?: () => void;
	tabIndex?: number;
	forwardedRef?: Ref<HTMLDivElement>;
	onKeyDown?: (e: KeyboardEvent) => void;
	isOpen?: boolean;
	dragDisabled?: boolean;
};

export const KanbanCard = ({
	id,
	name,
	index,
	parent,
	children,
	className,
	onClick,
	tabIndex,
	forwardedRef,
	onKeyDown,
	isOpen,
	dragDisabled = false,
}: KanbanCardProps) => {
	const { attributes, listeners, setNodeRef, transform, isDragging } =
		useDraggable({
			id,
			data: { index, parent },
			disabled: dragDisabled,
		});

	// Combine DnD ref and forwarded ref
	const combinedRef = (node: HTMLDivElement | null) => {
		setNodeRef(node);
		if (typeof forwardedRef === "function") {
			forwardedRef(node);
		} else if (forwardedRef && typeof forwardedRef === "object") {
			(forwardedRef as React.MutableRefObject<HTMLDivElement | null>).current =
				node;
		}
	};

	return (
		<Card
			className={cn(
				"p-3 outline-none border-b flex-col space-y-2",
				isDragging && "cursor-grabbing",
				isOpen && "ring-2 ring-secondary-foreground ring-inset",
				className,
			)}
			{...listeners}
			{...attributes}
			ref={combinedRef}
			tabIndex={tabIndex}
			onClick={onClick}
			onKeyDown={onKeyDown}
			style={{
				zIndex: isDragging ? 1000 : 1,
				transform: transform
					? `translateX(${transform.x}px) translateY(${transform.y}px)`
					: "none",
			}}
		>
			{children ?? <p className="m-0 font-medium text-sm">{name}</p>}
		</Card>
	);
};

export type KanbanCardsProps = {
	children: ReactNode;
	className?: string;
};

export const KanbanCards = ({ children, className }: KanbanCardsProps) => (
	<div className={cn("flex flex-1 flex-col", className)}>{children}</div>
);

export type KanbanHeaderProps =
	| {
			children: ReactNode;
	  }
	| {
			name: Status["name"];
			color: Status["color"];
			className?: string;
			count?: number;
			onAddTask?: () => void;
			onRefresh?: () => void;
			isRefreshing?: boolean;
			onToggleCollapse?: () => void;
	  };

export const KanbanHeader = (props: KanbanHeaderProps) => {
	const { t } = useTranslation("tasks");

	if ("children" in props) {
		return props.children;
	}

	return (
		<Card
			className={cn(
				"sticky top-0 z-20 flex shrink-0 items-center gap-2 p-3 border-b border-dashed flex gap-2 group/header",
				"bg-background",
				props.className,
			)}
			style={{
				backgroundImage: `linear-gradient(hsl(var(${props.color}) / 0.03), hsl(var(${props.color}) / 0.03))`,
			}}
		>
			<span className="flex-1 flex items-center gap-2">
				<div
					className="h-2 w-2 rounded-full"
					style={{ backgroundColor: `hsl(var(${props.color}))` }}
				/>

				<p className="m-0 text-sm">
					{props.name}
					{props.count !== undefined && (
						<span className="ml-1 text-muted-foreground">({props.count})</span>
					)}
				</p>
			</span>
			{props.onToggleCollapse && (
				<TooltipProvider>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="ghost"
								className="m-0 p-0 h-0 text-foreground/50 hover:text-foreground opacity-0 group-hover/header:opacity-100 transition-opacity"
								onClick={props.onToggleCollapse}
								aria-label={t("actions.collapseColumn")}
							>
								<EyeOff className="h-4 w-4" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top">
							{t("actions.collapseColumn")}
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			)}
			{props.onRefresh && (
				<TooltipProvider>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="ghost"
								className="m-0 p-0 h-0 text-foreground/50 hover:text-foreground"
								onClick={props.onRefresh}
								disabled={props.isRefreshing}
								aria-label={t("actions.refreshBacklog")}
							>
								<RefreshCw
									className={cn(
										"h-4 w-4",
										props.isRefreshing && "animate-spin",
									)}
								/>
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top">
							{t("actions.refreshBacklog")}
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			)}
			<TooltipProvider>
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							variant="ghost"
							className="m-0 p-0 h-0 text-foreground/50 hover:text-foreground"
							onClick={props.onAddTask}
							aria-label={t("actions.addTask")}
						>
							<Plus className="h-4 w-4" />
						</Button>
					</TooltipTrigger>
					<TooltipContent side="top">{t("actions.addTask")}</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		</Card>
	);
};

export type KanbanBoardCollapsedProps = {
	id: Status["id"];
	name: Status["name"];
	color: Status["color"];
	count?: number;
	onExpand: () => void;
};

export const KanbanBoardCollapsed = ({
	id,
	name,
	color,
	count,
	onExpand,
}: KanbanBoardCollapsedProps) => {
	const { setNodeRef } = useDroppable({ id });
	const { t } = useTranslation("tasks");

	return (
		<TooltipProvider>
			<Tooltip>
				<TooltipTrigger asChild>
					<div
						ref={setNodeRef}
						className="flex flex-col items-center justify-start w-10 min-h-40 cursor-pointer hover:bg-accent/50 transition-colors border-r"
						onClick={onExpand}
						style={{
							backgroundImage: `linear-gradient(hsl(var(${color}) / 0.05), hsl(var(${color}) / 0.05))`,
						}}
					>
						<div className="pt-3 pb-2">
							<div
								className="h-2 w-2 rounded-full"
								style={{ backgroundColor: `hsl(var(${color}))` }}
							/>
						</div>
						<div className="flex-1 flex items-start">
							<span
								className="text-sm text-muted-foreground whitespace-nowrap"
								style={{
									writingMode: "vertical-rl",
									textOrientation: "mixed",
								}}
							>
								{name}
								{count !== undefined && ` (${count})`}
							</span>
						</div>
						<div className="pb-3">
							<Eye className="h-4 w-4 text-muted-foreground" />
						</div>
					</div>
				</TooltipTrigger>
				<TooltipContent side="right">
					{t("actions.expandColumn")}
				</TooltipContent>
			</Tooltip>
		</TooltipProvider>
	);
};

function restrictToBoundingRectWithRightPadding(
	transform: Transform,
	rect: ClientRect,
	boundingRect: ClientRect,
	rightPadding: number,
): Transform {
	const value = {
		...transform,
	};

	if (rect.top + transform.y <= boundingRect.top) {
		value.y = boundingRect.top - rect.top;
	} else if (
		rect.bottom + transform.y >=
		boundingRect.top + boundingRect.height
	) {
		value.y = boundingRect.top + boundingRect.height - rect.bottom;
	}

	if (rect.left + transform.x <= boundingRect.left) {
		value.x = boundingRect.left - rect.left;
	} else if (
		// branch that checks if the right edge of the dragged element is beyond
		// the right edge of the bounding rectangle
		rect.right + transform.x + rightPadding >=
		boundingRect.left + boundingRect.width
	) {
		value.x =
			boundingRect.left + boundingRect.width - rect.right - rightPadding;
	}

	return {
		...value,
		x: value.x,
	};
}

// An alternative to `restrictToFirstScrollableAncestor` from the dnd-kit library
const restrictToFirstScrollableAncestorCustom: Modifier = (args) => {
	const { draggingNodeRect, transform, scrollableAncestorRects } = args;
	const firstScrollableAncestorRect = scrollableAncestorRects[0];

	if (!draggingNodeRect || !firstScrollableAncestorRect) {
		return transform;
	}

	// Inset the right edge that the rect can be dragged to by this amount.
	// This is a workaround for the kanban board where dragging a card too far
	// to the right causes infinite horizontal scrolling if there are also
	// enough cards for vertical scrolling to be enabled.
	const rightPadding = 16;
	return restrictToBoundingRectWithRightPadding(
		transform,
		draggingNodeRect,
		firstScrollableAncestorRect,
		rightPadding,
	);
};

export type KanbanProviderProps = {
	children: ReactNode;
	onDragEnd: (event: DragEndEvent) => void;
	className?: string;
};

export const KanbanProvider = ({
	children,
	onDragEnd,
	className,
}: KanbanProviderProps) => {
	const sensors = useSensors(
		useSensor(PointerSensor, {
			activationConstraint: { distance: 8 },
		}),
	);

	return (
		<DndContext
			collisionDetection={rectIntersection}
			onDragEnd={onDragEnd}
			sensors={sensors}
			modifiers={[restrictToFirstScrollableAncestorCustom]}
		>
			<div
				className={cn(
					"inline-flex divide-x border-x items-stretch min-h-full",
					className,
				)}
			>
				{children}
			</div>
		</DndContext>
	);
};
