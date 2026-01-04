import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal, getErrorMessage } from "@/lib/modals";
import { useProjectMutations } from "@/hooks/useProjectMutations";

export interface SetDevScriptDialogProps {
	projectId: string;
	projectName: string;
}

export type SetDevScriptDialogResult = {
	action: "saved" | "canceled";
	devScript?: string;
};

const SetDevScriptDialogImpl = NiceModal.create<SetDevScriptDialogProps>(
	({ projectId, projectName }) => {
		const modal = useModal();
		const { t } = useTranslation(["tasks", "common", "settings"]);
		const [devScript, setDevScript] = useState<string>("");
		const [error, setError] = useState<string | null>(null);

		const { updateProject } = useProjectMutations({
			onUpdateSuccess: () => {
				modal.resolve({
					action: "saved",
					devScript,
				} as SetDevScriptDialogResult);
				modal.hide();
			},
			onUpdateError: (err) => {
				setError(getErrorMessage(err) || "Failed to save dev script");
			},
		});

		const handleSave = () => {
			const trimmed = devScript.trim();
			if (!trimmed) {
				setError("Dev script cannot be empty");
				return;
			}
			setError(null);
			updateProject.mutate({
				projectId,
				data: {
					name: null,
					dev_script: trimmed,
					dev_script_working_dir: null,
					default_agent_working_dir: null,
					linear_api_key: null,
				},
			});
		};

		const handleCancel = () => {
			modal.resolve({ action: "canceled" } as SetDevScriptDialogResult);
			modal.hide();
		};

		const handleOpenChange = (open: boolean) => {
			if (!open) {
				handleCancel();
			}
		};

		return (
			<Dialog open={modal.visible} onOpenChange={handleOpenChange}>
				<DialogContent className="sm:max-w-md">
					<DialogHeader>
						<DialogTitle>{t("attempt.setDevScript.title")}</DialogTitle>
						<DialogDescription>
							{t("attempt.setDevScript.description", { projectName })}
						</DialogDescription>
					</DialogHeader>

					<div className="space-y-4">
						<div className="space-y-2">
							<label htmlFor="dev-script" className="text-sm font-medium">
								{t("attempt.setDevScript.label")}
							</label>
							<Input
								id="dev-script"
								type="text"
								value={devScript}
								onChange={(e) => {
									setDevScript(e.target.value);
									setError(null);
								}}
								onKeyDown={(e) => {
									if (e.key === "Enter" && !updateProject.isPending) {
										handleSave();
									}
								}}
								placeholder="npm run dev"
								disabled={updateProject.isPending}
								autoFocus
							/>
							{error && <p className="text-sm text-destructive">{error}</p>}
						</div>
					</div>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={handleCancel}
							disabled={updateProject.isPending}
						>
							{t("common:buttons.cancel")}
						</Button>
						<Button
							onClick={handleSave}
							disabled={updateProject.isPending || !devScript.trim()}
						>
							{updateProject.isPending
								? t("common:buttons.saving")
								: t("common:buttons.save")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		);
	},
);

export const SetDevScriptDialog = defineModal<
	SetDevScriptDialogProps,
	SetDevScriptDialogResult
>(SetDevScriptDialogImpl);
