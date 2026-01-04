import { useState, useCallback, useRef } from "react";

const MAX_HISTORY_SIZE = 100;

/**
 * Hook to manage shell-style input history navigation.
 * Ctrl+P goes to previous message, Ctrl+N goes to next.
 */
export function useInputHistory() {
	// History array - most recent at end
	const [history, setHistory] = useState<string[]>([]);
	// Current position in history (-1 means not navigating, editing new message)
	const [historyIndex, setHistoryIndex] = useState(-1);
	// Store the current draft when user starts navigating
	const draftRef = useRef<string>("");

	/**
	 * Add a message to history (called when message is sent)
	 */
	const addToHistory = useCallback((message: string) => {
		if (!message.trim()) return;

		setHistory((prev) => {
			// Don't add duplicates of the last message
			if (prev.length > 0 && prev[prev.length - 1] === message) {
				return prev;
			}
			const newHistory = [...prev, message];
			// Limit history size
			if (newHistory.length > MAX_HISTORY_SIZE) {
				return newHistory.slice(-MAX_HISTORY_SIZE);
			}
			return newHistory;
		});
		// Reset navigation state
		setHistoryIndex(-1);
		draftRef.current = "";
	}, []);

	/**
	 * Navigate to previous message (Ctrl+P)
	 * Returns the message to display, or null if no change
	 */
	const goToPrevious = useCallback(
		(currentValue: string): string | null => {
			if (history.length === 0) return null;

			// If not currently navigating, save current input as draft
			if (historyIndex === -1) {
				draftRef.current = currentValue;
				const newIndex = history.length - 1;
				setHistoryIndex(newIndex);
				return history[newIndex];
			}

			// Already at oldest message
			if (historyIndex === 0) return null;

			const newIndex = historyIndex - 1;
			setHistoryIndex(newIndex);
			return history[newIndex];
		},
		[history, historyIndex],
	);

	/**
	 * Navigate to next message (Ctrl+N)
	 * Returns the message to display, or null if no change
	 */
	const goToNext = useCallback((): string | null => {
		// Not navigating history
		if (historyIndex === -1) return null;

		// At the end of history, restore draft
		if (historyIndex === history.length - 1) {
			setHistoryIndex(-1);
			return draftRef.current;
		}

		const newIndex = historyIndex + 1;
		setHistoryIndex(newIndex);
		return history[newIndex];
	}, [history, historyIndex]);

	/**
	 * Reset navigation state (call when user types)
	 */
	const resetNavigation = useCallback(() => {
		if (historyIndex !== -1) {
			setHistoryIndex(-1);
			draftRef.current = "";
		}
	}, [historyIndex]);

	return {
		history,
		historyIndex,
		addToHistory,
		goToPrevious,
		goToNext,
		resetNavigation,
		isNavigating: historyIndex !== -1,
	};
}
