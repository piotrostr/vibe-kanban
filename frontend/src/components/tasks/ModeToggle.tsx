import { memo, forwardRef, useEffect, useState, useCallback } from "react";
import { Pause, Play } from "lucide-react";
import { cn } from "@/lib/utils";

type Props = {
	isPlanMode: boolean;
	onToggle: () => void;
	disabled?: boolean;
	className?: string;
	showHint?: boolean;
};

const ModeToggleInner = forwardRef<HTMLButtonElement, Props>(
	({ isPlanMode, onToggle, disabled, className, showHint = true }, ref) => {
		const [isAnimating, setIsAnimating] = useState(false);

		useEffect(() => {
			setIsAnimating(true);
			const t = setTimeout(() => setIsAnimating(false), 300);
			return () => clearTimeout(t);
		}, [isPlanMode]);

		const handleClick = useCallback(() => {
			if (!disabled) {
				onToggle();
			}
		}, [disabled, onToggle]);

		return (
			<button
				ref={ref}
				type="button"
				onClick={handleClick}
				disabled={disabled}
				className={cn(
					"flex items-center gap-1.5 px-2 py-1 rounded text-sm transition-all",
					"hover:opacity-80 focus:outline-none focus:ring-2 focus:ring-offset-1",
					isAnimating && "scale-105",
					isPlanMode
						? "text-blue-600 dark:text-blue-400 focus:ring-blue-500"
						: "text-red-600 dark:text-red-500 focus:ring-red-500",
					disabled && "opacity-50 cursor-not-allowed",
					className,
				)}
			>
				{isPlanMode ? (
					<Pause className="h-3 w-3" />
				) : (
					<>
						<Play className="h-3 w-3" />
						<Play className="h-3 w-3 -ml-2" />
					</>
				)}
				<span className="font-medium">
					{isPlanMode ? "plan mode on" : "default"}
				</span>
				{showHint && (
					<span className="text-xs text-muted-foreground ml-1">
						(shift+tab to toggle)
					</span>
				)}
			</button>
		);
	},
);

ModeToggleInner.displayName = "ModeToggle";
export const ModeToggle = memo(ModeToggleInner);
