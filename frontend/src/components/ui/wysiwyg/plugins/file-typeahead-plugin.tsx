import { useState, useCallback, useRef } from "react";
import { createPortal } from "react-dom";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
	LexicalTypeaheadMenuPlugin,
	MenuOption,
} from "@lexical/react/LexicalTypeaheadMenuPlugin";
import { $createTextNode } from "lexical";
import { FileText } from "lucide-react";
import { projectsApi } from "@/lib/api";
import type { SearchResult } from "shared/types";

interface FileSearchResult extends SearchResult {
	name: string;
}

class FileOption extends MenuOption {
	file: FileSearchResult;

	constructor(file: FileSearchResult) {
		super(`file-${file.path}`);
		this.file = file;
	}
}

const MAX_DIALOG_HEIGHT = 320;
const VIEWPORT_MARGIN = 8;
const VERTICAL_GAP = 4;
const VERTICAL_GAP_ABOVE = 24;
const MIN_WIDTH = 320;

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

export function FileTypeaheadPlugin({ projectId }: { projectId?: string }) {
	const [editor] = useLexicalComposerContext();
	const [options, setOptions] = useState<FileOption[]>([]);
	const itemRefs = useRef<Map<number, HTMLDivElement>>(new Map());
	const lastSelectedIndexRef = useRef<number>(-1);

	const onQueryChange = useCallback(
		(query: string | null) => {
			if (query === null || !projectId) {
				setOptions([]);
				return;
			}

			if (query.length === 0) {
				setOptions([]);
				return;
			}

			projectsApi
				.searchFiles(projectId, query)
				.then((results) => {
					const fileResults: FileSearchResult[] = results.map((item) => ({
						...item,
						name: item.path.split("/").pop() || item.path,
					}));
					setOptions(fileResults.map((f) => new FileOption(f)));
				})
				.catch((err) => {
					console.error("Failed to search files", err);
				});
		},
		[projectId],
	);

	return (
		<LexicalTypeaheadMenuPlugin<FileOption>
			triggerFn={(text) => {
				const match = /(?:^|\s)@([^\s@]*)$/.exec(text);
				if (!match) return null;
				const offset = match.index + match[0].indexOf("@");
				return {
					leadOffset: offset,
					matchingString: match[1],
					replaceableString: match[0].slice(match[0].indexOf("@")),
				};
			}}
			options={options}
			onQueryChange={onQueryChange}
			onSelectOption={(option, nodeToReplace, closeMenu) => {
				editor.update(() => {
					const textToInsert = option.file.path;

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
					<div
						className="fixed bg-background border border-border rounded-md shadow-lg overflow-y-auto"
						style={{
							top,
							bottom,
							left,
							maxHeight,
							minWidth: MIN_WIDTH,
							zIndex: 10000,
						}}
					>
						{options.length === 0 ? (
							<div className="p-2 text-sm text-muted-foreground">
								No files found
							</div>
						) : (
							<div className="py-1">
								<div className="px-3 py-1 text-xs font-semibold text-muted-foreground uppercase">
									Files
								</div>
								{options.map((option, index) => {
									const file = option.file;
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
											<div className="flex items-center gap-2 font-medium truncate">
												<FileText className="h-3.5 w-3.5 text-muted-foreground flex-shrink-0" />
												<span>{file.name}</span>
											</div>
											<div className="text-xs text-muted-foreground truncate">
												{file.path}
											</div>
										</div>
									);
								})}
							</div>
						)}
					</div>,
					document.body,
				);
			}}
		/>
	);
}
