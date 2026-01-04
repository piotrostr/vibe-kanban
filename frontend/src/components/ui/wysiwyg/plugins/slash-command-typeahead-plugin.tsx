import { useState, useCallback, useRef, useMemo } from "react";
import { createPortal } from "react-dom";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
	LexicalTypeaheadMenuPlugin,
	MenuOption,
} from "@lexical/react/LexicalTypeaheadMenuPlugin";
import { $createTextNode } from "lexical";
import { Terminal } from "lucide-react";
import { useSlashCommands } from "@/hooks/useSlashCommands";
import type { SlashCommand } from "shared/types";

class SlashCommandOption extends MenuOption {
	command: SlashCommand;

	constructor(command: SlashCommand) {
		super(`slash-${command.qualified_name}`);
		this.command = command;
	}
}

const MAX_DIALOG_HEIGHT = 320;
const VIEWPORT_MARGIN = 8;
const VERTICAL_GAP = 4;
const VERTICAL_GAP_ABOVE = 24;
const MIN_WIDTH = 360;

function getMenuPosition(anchorEl: HTMLElement) {
	const rect = anchorEl.getBoundingClientRect();
	const viewportHeight = window.innerHeight;
	const viewportWidth = window.innerWidth;

	const spaceAbove = rect.top;
	const spaceBelow = viewportHeight - rect.bottom;

	const showBelow = spaceBelow >= spaceAbove;

	const availableVerticalSpace = showBelow ? spaceBelow : spaceAbove;

	const maxHeight = Math.max(
		0,
		Math.min(MAX_DIALOG_HEIGHT, availableVerticalSpace - 2 * VIEWPORT_MARGIN),
	);

	let top: number | undefined;
	let bottom: number | undefined;

	if (showBelow) {
		top = rect.bottom + VERTICAL_GAP;
	} else {
		bottom = viewportHeight - rect.top + VERTICAL_GAP_ABOVE;
	}

	let left = rect.left;
	const maxLeft = viewportWidth - MIN_WIDTH - VIEWPORT_MARGIN;
	if (left > maxLeft) {
		left = Math.max(VIEWPORT_MARGIN, maxLeft);
	}

	return { top, bottom, left, maxHeight };
}

// Separate component for rendering the menu
function SlashCommandMenu({
	options,
	selectedIndex,
	setHighlightedIndex,
	selectOptionAndCleanUp,
	itemRefs,
	style,
}: {
	options: SlashCommandOption[];
	selectedIndex: number | null;
	setHighlightedIndex: (index: number) => void;
	selectOptionAndCleanUp: (option: SlashCommandOption) => void;
	itemRefs: React.MutableRefObject<Map<number, HTMLDivElement>>;
	style: {
		top?: number;
		bottom?: number;
		left: number;
		maxHeight: number;
		minWidth: number;
	};
}) {
	return (
		<div
			className="fixed bg-background border border-border rounded-md shadow-lg overflow-y-auto"
			style={{
				top: style.top,
				bottom: style.bottom,
				left: style.left,
				maxHeight: style.maxHeight,
				minWidth: style.minWidth,
				zIndex: 10000,
			}}
		>
			{options.length === 0 ? (
				<div className="p-2 text-sm text-muted-foreground">
					No commands found
				</div>
			) : (
				<div className="py-1">
					<div className="px-3 py-1 text-xs font-semibold text-muted-foreground uppercase">
						Commands
					</div>
					{options.map((option, index) => {
						const cmd = option.command;
						return (
							<div
								key={option.key}
								ref={(el) => {
									if (el) itemRefs.current.set(index, el);
									else itemRefs.current.delete(index);
								}}
								className={`px-3 py-2 cursor-pointer text-sm ${
									index === selectedIndex
										? "bg-muted text-foreground"
										: "hover:bg-muted"
								}`}
								onMouseEnter={() => setHighlightedIndex(index)}
								onClick={() => selectOptionAndCleanUp(option)}
							>
								<div className="flex items-center gap-2 font-medium text-foreground">
									<Terminal className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
									<span>/{cmd.name}</span>
									{cmd.argument_hint && (
										<span className="text-muted-foreground font-normal">
											{cmd.argument_hint}
										</span>
									)}
								</div>
								{cmd.description && (
									<div className="text-xs text-muted-foreground mt-0.5 ml-5.5 truncate">
										{cmd.description}
									</div>
								)}
								{(cmd.plugin_name || cmd.source === "builtin") && (
									<div className="text-xs text-muted-foreground/60 mt-0.5 ml-5.5">
										{cmd.source === "builtin" ? "built-in" : cmd.plugin_name}
									</div>
								)}
							</div>
						);
					})}
				</div>
			)}
		</div>
	);
}

export function SlashCommandTypeaheadPlugin() {
	const [editor] = useLexicalComposerContext();
	const { commands } = useSlashCommands();
	const [options, setOptions] = useState<SlashCommandOption[]>([]);
	const itemRefs = useRef<Map<number, HTMLDivElement>>(new Map());
	const lastSelectedIndexRef = useRef<number>(-1);

	// Memoize command options for filtering
	const allCommandOptions = useMemo(
		() => commands.map((cmd) => new SlashCommandOption(cmd)),
		[commands],
	);

	const onQueryChange = useCallback(
		(query: string | null) => {
			if (query === null) {
				setOptions([]);
				return;
			}

			// Filter commands based on query
			const lowerQuery = query.toLowerCase();
			const filtered = allCommandOptions.filter((opt) => {
				const cmd = opt.command;
				return (
					cmd.name.toLowerCase().includes(lowerQuery) ||
					cmd.qualified_name.toLowerCase().includes(lowerQuery) ||
					(cmd.description?.toLowerCase().includes(lowerQuery) ?? false)
				);
			});

			setOptions(filtered);
		},
		[allCommandOptions],
	);

	return (
		<LexicalTypeaheadMenuPlugin<SlashCommandOption>
			triggerFn={(text) => {
				// Match / at start of line or after whitespace
				const match = /(?:^|\s)\/([^\s/]*)$/.exec(text);
				if (!match) return null;
				const offset = match.index + match[0].indexOf("/");
				return {
					leadOffset: offset,
					matchingString: match[1],
					replaceableString: match[0].slice(match[0].indexOf("/")),
				};
			}}
			options={options}
			onQueryChange={onQueryChange}
			onSelectOption={(option, nodeToReplace, closeMenu) => {
				editor.update(() => {
					// Insert the slash command - use name for built-ins, qualified_name for plugins
					const cmd = option.command;
					const commandText =
						cmd.source === "builtin" ? cmd.name : cmd.qualified_name;
					const textToInsert = `/${commandText} `;

					if (!nodeToReplace) return;

					const textNode = $createTextNode(textToInsert);
					nodeToReplace.replace(textNode);
					textNode.select(textToInsert.length, textToInsert.length);
				});

				closeMenu();
			}}
			menuRenderFn={(
				anchorRef,
				{ selectedIndex, selectOptionAndCleanUp, setHighlightedIndex },
			) => {
				if (!anchorRef.current) return null;

				const { top, bottom, left, maxHeight } = getMenuPosition(
					anchorRef.current,
				);

				if (
					selectedIndex !== null &&
					selectedIndex !== lastSelectedIndexRef.current
				) {
					lastSelectedIndexRef.current = selectedIndex;
					setTimeout(() => {
						const itemEl = itemRefs.current.get(selectedIndex);
						if (itemEl) {
							itemEl.scrollIntoView({ block: "nearest" });
						}
					}, 0);
				}

				return createPortal(
					<SlashCommandMenu
						options={options}
						selectedIndex={selectedIndex}
						setHighlightedIndex={setHighlightedIndex}
						selectOptionAndCleanUp={selectOptionAndCleanUp}
						itemRefs={itemRefs}
						style={{ top, bottom, left, maxHeight, minWidth: MIN_WIDTH }}
					/>,
					document.body,
				);
			}}
		/>
	);
}
