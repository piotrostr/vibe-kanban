import { useEffect } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
	KEY_MODIFIER_COMMAND,
	KEY_ENTER_COMMAND,
	KEY_TAB_COMMAND,
	COMMAND_PRIORITY_NORMAL,
	COMMAND_PRIORITY_HIGH,
} from "lexical";

type Props = {
	onCmdEnter?: () => void;
	onShiftCmdEnter?: () => void;
	onShiftTab?: () => void;
};

export function KeyboardCommandsPlugin({
	onCmdEnter,
	onShiftCmdEnter,
	onShiftTab,
}: Props) {
	const [editor] = useLexicalComposerContext();

	useEffect(() => {
		const unregisters: (() => void)[] = [];

		// Handle Cmd/Ctrl+Enter and Cmd/Ctrl+Shift+Enter
		if (onCmdEnter || onShiftCmdEnter) {
			unregisters.push(
				editor.registerCommand(
					KEY_MODIFIER_COMMAND,
					(event: KeyboardEvent) => {
						if (!(event.metaKey || event.ctrlKey) || event.key !== "Enter") {
							return false;
						}

						event.preventDefault();
						event.stopPropagation();

						if (event.shiftKey && onShiftCmdEnter) {
							onShiftCmdEnter();
							return true;
						}

						if (!event.shiftKey && onCmdEnter) {
							onCmdEnter();
							return true;
						}

						return false;
					},
					COMMAND_PRIORITY_NORMAL,
				),
			);

			// Block KEY_ENTER_COMMAND when CMD/Ctrl is pressed to prevent
			// RichTextPlugin from inserting a new line
			unregisters.push(
				editor.registerCommand(
					KEY_ENTER_COMMAND,
					(event: KeyboardEvent | null) => {
						if (event && (event.metaKey || event.ctrlKey)) {
							return true; // Mark as handled, preventing line break insertion
						}
						return false;
					},
					COMMAND_PRIORITY_HIGH,
				),
			);
		}

		// Handle Shift+Tab for mode toggle
		if (onShiftTab) {
			unregisters.push(
				editor.registerCommand(
					KEY_TAB_COMMAND,
					(event: KeyboardEvent) => {
						if (event.shiftKey) {
							event.preventDefault();
							event.stopPropagation();
							onShiftTab();
							return true;
						}
						return false;
					},
					COMMAND_PRIORITY_HIGH,
				),
			);
		}

		return () => {
			unregisters.forEach((unregister) => unregister());
		};
	}, [editor, onCmdEnter, onShiftCmdEnter, onShiftTab]);

	return null;
}
