import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AutoExpandingTextarea } from "@/components/ui/auto-expanding-textarea";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal, getErrorMessage } from "@/lib/modals";
import { projectsApi } from "@/lib/api";
import { useMutation, useQueryClient } from "@tanstack/react-query";

export interface SetSetupScriptDialogProps {
	projectId: string;
	repoId: string;
	repoName: string;
}

export type SetSetupScriptDialogResult = {
	action: "saved" | "canceled";
	setupScript?: string;
};

const SetSetupScriptDialogImpl = NiceModal.create<SetSetupScriptDialogProps>(
	({ projectId, repoId, repoName }) => {
		const modal = useModal();
		const { t } = useTranslation(["tasks", "common", "settings"]);
		const queryClient = useQueryClient();
		const [setupScript, setSetupScript] = useState<string>("");
		const [error, setError] = useState<string | null>(null);

		// Fetch current ProjectRepo to get existing setup_script
		const { data: projectRepo, isLoading } = useQuery({
			queryKey: ["projectRepo", projectId, repoId],
			queryFn: () => projectsApi.getRepository(projectId, repoId),
			enabled: modal.visible,
		});

		// Initialize setup script from fetched data
		useEffect(() => {
			if (projectRepo?.setup_script) {
				setSetupScript(projectRepo.setup_script);
			}
		}, [projectRepo]);

		const updateMutation = useMutation({
			mutationFn: () =>
				projectsApi.updateRepository(projectId, repoId, {
					setup_script: setupScript.trim() || null,
					cleanup_script: projectRepo?.cleanup_script ?? null,
					copy_files: projectRepo?.copy_files ?? null,
					parallel_setup_script: projectRepo?.parallel_setup_script ?? false,
				}),
			onSuccess: () => {
				queryClient.invalidateQueries({
					queryKey: ["projectRepo", projectId, repoId],
				});
				modal.resolve({
					action: "saved",
					setupScript,
				} as SetSetupScriptDialogResult);
				modal.hide();
			},
			onError: (err) => {
				setError(getErrorMessage(err) || "Failed to save setup script");
			},
		});

		const handleSave = () => {
			const trimmed = setupScript.trim();
			if (!trimmed) {
				setError("Setup script cannot be empty");
				return;
			}
			setError(null);
			updateMutation.mutate();
		};

		const handleCancel = () => {
			modal.resolve({ action: "canceled" } as SetSetupScriptDialogResult);
			modal.hide();
		};

		const handleOpenChange = (open: boolean) => {
			if (!open) {
				handleCancel();
			}
		};

		return (
			<Dialog open={modal.visible} onOpenChange={handleOpenChange}>
				<DialogContent className="sm:max-w-lg">
					<DialogHeader>
						<DialogTitle>{t("followUp.setSetupScript.title")}</DialogTitle>
						<DialogDescription>
							{t("followUp.setSetupScript.description", { repoName })}
						</DialogDescription>
					</DialogHeader>

					<div className="space-y-4">
						<div className="space-y-2">
							<label htmlFor="setup-script" className="text-sm font-medium">
								{t("followUp.setSetupScript.label")}
							</label>
							<AutoExpandingTextarea
								id="setup-script"
								value={setupScript}
								onChange={(e) => {
									setSetupScript(e.target.value);
									setError(null);
								}}
								placeholder="npm install"
								disabled={updateMutation.isPending || isLoading}
								maxRows={8}
								className="w-full px-3 py-2 border border-input bg-background text-foreground rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono text-sm"
							/>
							{error && <p className="text-sm text-destructive">{error}</p>}
						</div>
					</div>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={handleCancel}
							disabled={updateMutation.isPending}
						>
							{t("common:buttons.cancel")}
						</Button>
						<Button
							onClick={handleSave}
							disabled={
								updateMutation.isPending || isLoading || !setupScript.trim()
							}
						>
							{updateMutation.isPending
								? t("common:buttons.saving")
								: t("common:buttons.save")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		);
	},
);

export const SetSetupScriptDialog = defineModal<
	SetSetupScriptDialogProps,
	SetSetupScriptDialogResult
>(SetSetupScriptDialogImpl);
