import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { $getRoot, $createParagraphNode, $createTextNode } from "lexical";

type Props = {
	onPrevious: (currentValue: string) => string | null;
	onNext: () => string | null;
	onUserInput: () => void;
	getCurrentValue: () => string;
};

/**
 * Plugin for shell-style input history navigation.
 * Ctrl+P goes to previous message, Ctrl+N goes to next.
 */
export function InputHistoryPlugin({
	onPrevious,
	onNext,
	onUserInput,
	getCurrentValue,
}: Props) {
	const [editor] = useLexicalComposerContext();

	useEffect(() => {
		const handleKeyDown = (e: KeyboardEvent) => {
			// Only handle Ctrl+P and Ctrl+N (not Cmd on Mac - that's for other shortcuts)
			if (!e.ctrlKey || e.metaKey) return;

			if (e.key === "p" || e.key === "P") {
				e.preventDefault();
				e.stopPropagation();

				const newValue = onPrevious(getCurrentValue());
				if (newValue !== null) {
					editor.update(() => {
						const root = $getRoot();
						root.clear();
						const paragraph = $createParagraphNode();
						paragraph.append($createTextNode(newValue));
						root.append(paragraph);
						// Move cursor to end
						root.selectEnd();
					});
				}
			} else if (e.key === "n" || e.key === "N") {
				e.preventDefault();
				e.stopPropagation();

				const newValue = onNext();
				if (newValue !== null) {
					editor.update(() => {
						const root = $getRoot();
						root.clear();
						const paragraph = $createParagraphNode();
						paragraph.append($createTextNode(newValue));
						root.append(paragraph);
						// Move cursor to end
						root.selectEnd();
					});
				}
			}
		};

		// Use capture phase to intercept before other handlers
		document.addEventListener("keydown", handleKeyDown, true);
		return () => {
			document.removeEventListener("keydown", handleKeyDown, true);
		};
	}, [editor, onPrevious, onNext, getCurrentValue]);

	// Listen for text changes to reset navigation
	useEffect(() => {
		return editor.registerTextContentListener(() => {
			onUserInput();
		});
	}, [editor, onUserInput]);

	return null;
}
