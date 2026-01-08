import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import { useTranslation } from "react-i18next";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import WYSIWYGEditor from "@/components/ui/wysiwyg";

export interface ViewPlanDialogProps {
	planMarkdown: string;
}

const ViewPlanDialogImpl = NiceModal.create<ViewPlanDialogProps>(
	({ planMarkdown }) => {
		const { t } = useTranslation("tasks");
		const modal = useModal();

		const handleOpenChange = (open: boolean) => {
			if (!open) {
				modal.hide();
			}
		};

		return (
			<Dialog
				open={modal.visible}
				onOpenChange={handleOpenChange}
				className="max-w-4xl w-[90vw] p-0 overflow-hidden"
			>
				<DialogContent
					className="p-0 min-w-0 flex flex-col max-h-[85vh]"
					onKeyDownCapture={(e) => {
						if (e.key === "Escape") {
							e.stopPropagation();
							modal.hide();
						}
					}}
				>
					<DialogHeader className="px-4 py-3 border-b shrink-0">
						<DialogTitle>{t("viewPlanDialog.title")}</DialogTitle>
					</DialogHeader>
					<div className="flex-1 overflow-y-auto px-4 py-4">
						<WYSIWYGEditor
							value={planMarkdown}
							disabled
							className="whitespace-pre-wrap break-words prose prose-sm dark:prose-invert max-w-none"
						/>
					</div>
				</DialogContent>
			</Dialog>
		);
	},
);

export const ViewPlanDialog = defineModal<ViewPlanDialogProps, void>(
	ViewPlanDialogImpl,
);
