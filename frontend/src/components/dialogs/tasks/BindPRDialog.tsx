import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@radix-ui/react-label";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Alert } from "@/components/ui/alert";
import { useCallback, useEffect, useState } from "react";
import { attemptsApi } from "@/lib/api";
import { useTranslation } from "react-i18next";
import { Loader2, Link } from "lucide-react";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";

interface BindPRDialogProps {
	attemptId: string;
	repoId: string;
}

const BindPRDialogImpl = NiceModal.create<BindPRDialogProps>(
	({ attemptId, repoId }) => {
		const modal = useModal();
		const { t } = useTranslation("tasks");
		const [prNumber, setPrNumber] = useState("");
		const [binding, setBinding] = useState(false);
		const [error, setError] = useState<string | null>(null);

		useEffect(() => {
			if (modal.visible) {
				setPrNumber("");
				setError(null);
			}
		}, [modal.visible]);

		const handleConfirmBind = useCallback(async () => {
			if (!repoId || !attemptId) return;

			const parsedPrNumber = parseInt(prNumber, 10);
			if (isNaN(parsedPrNumber) || parsedPrNumber <= 0) {
				setError(t("bindPrDialog.errors.invalidNumber"));
				return;
			}

			setError(null);
			setBinding(true);

			const result = await attemptsApi.bindPR(attemptId, {
				repo_id: repoId,
				pr_number: BigInt(parsedPrNumber),
			});

			setBinding(false);

			if (result.success) {
				modal.resolve(result.data);
				modal.hide();
				return;
			}

			if (result.error) {
				switch (result.error.type) {
					case "pr_not_found":
						setError(
							t("bindPrDialog.errors.prNotFound", {
								number: Number(result.error.pr_number),
							}),
						);
						break;
					case "github_cli_not_installed":
						setError(t("bindPrDialog.errors.githubCliNotInstalled"));
						break;
					case "github_cli_not_logged_in":
						setError(t("bindPrDialog.errors.githubCliNotLoggedIn"));
						break;
					default:
						setError(result.message || t("bindPrDialog.errors.failedToBind"));
				}
			} else {
				setError(result.message || t("bindPrDialog.errors.failedToBind"));
			}
		}, [attemptId, repoId, prNumber, modal, t]);

		const handleCancel = useCallback(() => {
			modal.reject("canceled");
			modal.hide();
			setPrNumber("");
		}, [modal]);

		const handleKeyDown = useCallback(
			(e: React.KeyboardEvent) => {
				if (e.key === "Enter" && prNumber.trim()) {
					handleConfirmBind();
				}
			},
			[prNumber, handleConfirmBind],
		);

		return (
			<Dialog open={modal.visible} onOpenChange={() => handleCancel()}>
				<DialogContent className="sm:max-w-[400px]">
					<DialogHeader>
						<DialogTitle>{t("bindPrDialog.title")}</DialogTitle>
						<DialogDescription>
							{t("bindPrDialog.description")}
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4 py-4">
						<div className="space-y-2">
							<Label htmlFor="pr-number">
								{t("bindPrDialog.prNumberLabel")}
							</Label>
							<div className="relative">
								<span className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground">
									#
								</span>
								<Input
									id="pr-number"
									type="number"
									min="1"
									value={prNumber}
									onChange={(e) => setPrNumber(e.target.value)}
									onKeyDown={handleKeyDown}
									placeholder={t("bindPrDialog.prNumberPlaceholder")}
									className="pl-7"
									autoFocus
								/>
							</div>
						</div>
						{error && <Alert variant="destructive">{error}</Alert>}
					</div>
					<DialogFooter>
						<Button variant="outline" onClick={handleCancel}>
							{t("common:buttons.cancel")}
						</Button>
						<Button
							onClick={handleConfirmBind}
							disabled={binding || !prNumber.trim()}
							className="bg-blue-600 hover:bg-blue-700"
						>
							{binding ? (
								<>
									<Loader2 className="mr-2 h-4 w-4 animate-spin" />
									{t("bindPrDialog.binding")}
								</>
							) : (
								<>
									<Link className="mr-2 h-4 w-4" />
									{t("bindPrDialog.bindButton")}
								</>
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		);
	},
);

export const BindPRDialog = defineModal<BindPRDialogProps, void>(
	BindPRDialogImpl,
);
