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
	/** Plain Enter key - sends message (Claude Code style) */
	onEnter?: () => void;
	onCmdEnter?: () => void;
	onShiftCmdEnter?: () => void;
	onShiftTab?: () => void;
};

export function KeyboardCommandsPlugin({
	onEnter,
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
		}

		// Handle Enter key - Claude Code style:
		// - Plain Enter: send message (call onEnter)
		// - Alt/Option+Enter: insert newline (allow default behavior)
		// - Cmd/Ctrl+Enter: handled above by KEY_MODIFIER_COMMAND
		if (onEnter) {
			unregisters.push(
				editor.registerCommand(
					KEY_ENTER_COMMAND,
					(event: KeyboardEvent | null) => {
						if (!event) return false;

						// Alt/Option+Enter: allow newline insertion
						if (event.altKey) {
							return false;
						}

						// Cmd/Ctrl+Enter: handled by KEY_MODIFIER_COMMAND
						if (event.metaKey || event.ctrlKey) {
							return true; // Block default, let modifier handler deal with it
						}

						// Plain Enter: send message
						event.preventDefault();
						event.stopPropagation();
						onEnter();
						return true;
					},
					COMMAND_PRIORITY_HIGH,
				),
			);
		} else if (onCmdEnter || onShiftCmdEnter) {
			// Legacy: Block KEY_ENTER_COMMAND when CMD/Ctrl is pressed
			unregisters.push(
				editor.registerCommand(
					KEY_ENTER_COMMAND,
					(event: KeyboardEvent | null) => {
						if (event && (event.metaKey || event.ctrlKey)) {
							return true;
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
	}, [editor, onEnter, onCmdEnter, onShiftCmdEnter, onShiftTab]);

	return null;
}
