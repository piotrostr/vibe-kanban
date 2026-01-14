import { useState, useEffect, useMemo, useCallback } from "react";
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
import RepoBranchSelector from "@/components/tasks/RepoBranchSelector";
import { ModeToggle } from "@/components/tasks/ModeToggle";
import { useAttemptCreation } from "@/hooks/useAttemptCreation";
import {
	useNavigateWithSearch,
	useTask,
	useAttempt,
	useRepoBranchSelection,
	useProjectRepos,
} from "@/hooks";
import { useTaskAttemptsWithSessions } from "@/hooks/useTaskAttempts";
import { paths } from "@/lib/paths";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import { BaseCodingAgent } from "shared/types";
import type { ExecutorProfileId } from "shared/types";
import { useKeySubmitTask, useKeyToggleMode, Scope } from "@/keyboard";
import { RepoPickerDialog } from "@/components/dialogs/shared/RepoPickerDialog";
import { projectsApi } from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { FolderGit } from "lucide-react";

export interface CreateAttemptDialogProps {
	taskId: string;
	projectId: string;
}

const CreateAttemptDialogImpl = NiceModal.create<CreateAttemptDialogProps>(
	({ taskId, projectId }) => {
		const modal = useModal();
		const navigate = useNavigateWithSearch();
		const { t } = useTranslation("tasks");
		const queryClient = useQueryClient();
		const { createAttempt, isCreating, error } = useAttemptCreation({
			taskId,
			onSuccess: (attempt) => {
				if (projectId) {
					navigate(paths.attempt(projectId, taskId, attempt.id));
				}
			},
		});

		const [isPlanMode, setIsPlanMode] = useState(false);

		const { isLoading: isLoadingAttempts } = useTaskAttemptsWithSessions(
			taskId,
			{
				enabled: modal.visible,
				refetchInterval: 5000,
			},
		);

		const { data: task, isLoading: isLoadingTask } = useTask(taskId, {
			enabled: modal.visible,
		});

		const parentAttemptId = task?.parent_workspace_id ?? undefined;
		const { data: parentAttempt, isLoading: isLoadingParent } = useAttempt(
			parentAttemptId,
			{ enabled: modal.visible && !!parentAttemptId },
		);

		const { data: projectRepos = [], isLoading: isLoadingRepos } =
			useProjectRepos(projectId, { enabled: modal.visible });

		const {
			configs: repoBranchConfigs,
			isLoading: isLoadingBranches,
			setRepoBranch,
			getWorkspaceRepoInputs,
			reset: resetBranchSelection,
		} = useRepoBranchSelection({
			repos: projectRepos,
			initialBranch: parentAttempt?.branch,
			enabled: modal.visible && projectRepos.length > 0,
		});

		useEffect(() => {
			if (!modal.visible) {
				setIsPlanMode(false);
				resetBranchSelection();
			}
		}, [modal.visible, resetBranchSelection]);

		// Always use CLAUDE_CODE with the selected mode
		const effectiveProfile: ExecutorProfileId = useMemo(
			() => ({
				executor: BaseCodingAgent.CLAUDE_CODE,
				variant: isPlanMode ? "PLAN" : null,
			}),
			[isPlanMode],
		);

		const handleToggleMode = useCallback(() => {
			setIsPlanMode((prev) => !prev);
		}, []);

		const isLoadingInitial =
			isLoadingRepos ||
			isLoadingBranches ||
			isLoadingAttempts ||
			isLoadingTask ||
			isLoadingParent;

		const allBranchesSelected = repoBranchConfigs.every(
			(c) => c.targetBranch !== null,
		);

		const canCreate = Boolean(
			allBranchesSelected &&
				projectRepos.length > 0 &&
				!isCreating &&
				!isLoadingInitial,
		);

		const handleCreate = async () => {
			if (!allBranchesSelected || projectRepos.length === 0) return;
			try {
				const repos = getWorkspaceRepoInputs();

				await createAttempt({
					profile: effectiveProfile,
					repos,
				});

				modal.hide();
			} catch (err) {
				console.error("Failed to create attempt:", err);
			}
		};

		const handleOpenChange = (open: boolean) => {
			if (!open) modal.hide();
		};

		const handleAddRepo = useCallback(async () => {
			if (!projectId) return;

			const repo = await RepoPickerDialog.show({
				title: t("createAttemptDialog.selectRepo"),
				description: t("createAttemptDialog.selectRepoDescription"),
			});

			if (!repo) return;

			try {
				await projectsApi.addRepository(projectId, {
					display_name: repo.display_name,
					git_repo_path: repo.path,
				});
				queryClient.invalidateQueries({
					queryKey: ["projectRepositories", projectId],
				});
			} catch (err) {
				console.error("Failed to add repository:", err);
			}
		}, [projectId, queryClient, t]);

		useKeySubmitTask(handleCreate, {
			enabled: modal.visible && canCreate,
			scope: Scope.DIALOG,
			preventDefault: true,
		});

		useKeyToggleMode(handleToggleMode, {
			scope: Scope.DIALOG,
			enabled: modal.visible,
		});

		return (
			<Dialog open={modal.visible} onOpenChange={handleOpenChange}>
				<DialogContent className="sm:max-w-[500px]">
					<DialogHeader>
						<DialogTitle>{t("createAttemptDialog.title")}</DialogTitle>
						<DialogDescription>
							{t("createAttemptDialog.description")}
						</DialogDescription>
					</DialogHeader>

					<div className="space-y-4 py-4">
						<div className="space-y-2">
							<ModeToggle isPlanMode={isPlanMode} onToggle={handleToggleMode} />
						</div>

						{projectRepos.length > 0 ? (
							<RepoBranchSelector
								configs={repoBranchConfigs}
								onBranchChange={setRepoBranch}
								isLoading={isLoadingBranches}
								className="space-y-2"
							/>
						) : (
							!isLoadingRepos && (
								<div
									className="p-4 border border-dashed cursor-pointer hover:bg-muted/50 transition-colors rounded-lg"
									onClick={handleAddRepo}
								>
									<div className="flex items-start gap-3">
										<FolderGit className="h-5 w-5 mt-0.5 flex-shrink-0 text-muted-foreground" />
										<div className="min-w-0 flex-1">
											<div className="font-medium text-foreground">
												{t("createAttemptDialog.addRepo")}
											</div>
											<div className="text-xs text-muted-foreground mt-1">
												{t("createAttemptDialog.addRepoDescription")}
											</div>
										</div>
									</div>
								</div>
							)
						)}

						{error && (
							<div className="text-sm text-destructive">
								{t("createAttemptDialog.error")}
							</div>
						)}
					</div>

					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => modal.hide()}
							disabled={isCreating}
						>
							{t("common:buttons.cancel")}
						</Button>
						<Button onClick={handleCreate} disabled={!canCreate}>
							{isCreating
								? t("createAttemptDialog.creating")
								: t("createAttemptDialog.start")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		);
	},
);

export const CreateAttemptDialog = defineModal<CreateAttemptDialogProps, void>(
	CreateAttemptDialogImpl,
);
