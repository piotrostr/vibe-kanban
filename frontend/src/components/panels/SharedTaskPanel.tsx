import type { SharedTaskRecord } from "@/hooks/useProjectTasks";
import { NewCardContent } from "@/components/ui/new-card";
import WYSIWYGEditor from "@/components/ui/wysiwyg";
import { usePrivacy } from "@/contexts/PrivacyContext";
import { maskText } from "@/lib/privacyMask";

interface SharedTaskPanelProps {
	task: SharedTaskRecord;
}

const SharedTaskPanel = ({ task }: SharedTaskPanelProps) => {
	const { privacyMode } = usePrivacy();
	const displayTitle = privacyMode ? maskText(task.title) : task.title;
	const displayDescription = privacyMode
		? maskText(task.description)
		: task.description;

	return (
		<NewCardContent>
			<div className="p-6 flex flex-col gap-6">
				<div className="flex items-start justify-between gap-3">
					<div className="flex-1 min-w-0">
						<h1 className="text-xl font-semibold leading-tight break-words">
							{displayTitle}
						</h1>
					</div>
				</div>
				{task.description ? (
					<WYSIWYGEditor value={displayDescription ?? ""} disabled />
				) : null}
			</div>
		</NewCardContent>
	);
};

export default SharedTaskPanel;
