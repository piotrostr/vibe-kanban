import { useCallback } from "react";
import { attemptsApi } from "@/lib/api";
import { EditorSelectionDialog } from "@/components/dialogs/tasks/EditorSelectionDialog";
import type { EditorType } from "shared/types";
import { openExternal } from "@/lib/openExternal";

type OpenEditorOptions = {
	editorType?: EditorType;
	filePath?: string;
};

export function useOpenInEditor(
	attemptId?: string,
	onShowEditorDialog?: () => void,
) {
	return useCallback(
		async (options?: OpenEditorOptions): Promise<void> => {
			if (!attemptId) return;

			const { editorType, filePath } = options ?? {};

			try {
				const response = await attemptsApi.openEditor(attemptId, {
					editor_type: editorType ?? null,
					file_path: filePath ?? null,
				});

				// If a URL is returned, open it in the external browser
				if (response.url) {
					await openExternal(response.url);
				}
			} catch (err) {
				console.error("Failed to open editor:", err);
				if (!editorType) {
					if (onShowEditorDialog) {
						onShowEditorDialog();
					} else {
						EditorSelectionDialog.show({
							selectedAttemptId: attemptId,
							filePath,
						});
					}
				}
			}
		},
		[attemptId, onShowEditorDialog],
	);
}
