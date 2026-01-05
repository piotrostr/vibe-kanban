import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { TaskStatus } from "shared/types";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import { statusLabels } from "@/utils/statusLabels";
import { ArrowRight } from "lucide-react";

export interface LinearSyncConfirmDialogProps {
	taskTitle: string;
	fromStatus: TaskStatus;
	toStatus: TaskStatus;
}

export type LinearSyncConfirmResult = "sync" | "local-only" | "cancelled";

const LinearSyncConfirmDialogImpl =
	NiceModal.create<LinearSyncConfirmDialogProps>(
		({ taskTitle, fromStatus, toStatus }) => {
			const modal = useModal();

			const handleSync = () => {
				modal.resolve("sync" as LinearSyncConfirmResult);
				modal.hide();
			};

			const handleLocalOnly = () => {
				modal.resolve("local-only" as LinearSyncConfirmResult);
				modal.hide();
			};

			const handleCancel = () => {
				modal.resolve("cancelled" as LinearSyncConfirmResult);
				modal.hide();
			};

			return (
				<Dialog
					open={modal.visible}
					onOpenChange={(open) => !open && handleCancel()}
				>
					<DialogContent className="sm:max-w-[425px]">
						<DialogHeader>
							<DialogTitle>Sync to Linear?</DialogTitle>
							<DialogDescription>
								This task is linked to Linear. Do you want to update its status
								in Linear as well?
							</DialogDescription>
						</DialogHeader>

						<div className="py-4">
							<p className="text-sm font-medium mb-2 truncate">{taskTitle}</p>
							<div className="flex items-center gap-2 text-sm text-muted-foreground">
								<span className="px-2 py-0.5 rounded bg-muted">
									{statusLabels[fromStatus]}
								</span>
								<ArrowRight className="h-4 w-4 shrink-0" />
								<span className="px-2 py-0.5 rounded bg-muted">
									{statusLabels[toStatus]}
								</span>
							</div>
						</div>

						<DialogFooter className="gap-2 sm:gap-0">
							<Button variant="outline" onClick={handleLocalOnly}>
								Local Only
							</Button>
							<Button onClick={handleSync}>Sync to Linear</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>
			);
		},
	);

export const LinearSyncConfirmDialog = defineModal<
	LinearSyncConfirmDialogProps,
	LinearSyncConfirmResult
>(LinearSyncConfirmDialogImpl);
