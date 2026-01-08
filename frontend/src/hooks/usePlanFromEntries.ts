import { useMemo } from "react";
import { useEntries } from "@/contexts/EntriesContext";

export function usePlanFromEntries(): string | null {
	const { entries } = useEntries();

	return useMemo(() => {
		for (const entry of entries) {
			if (entry.type !== "NORMALIZED_ENTRY") continue;

			const entryType = entry.content.entry_type;
			if (entryType.type !== "tool_use") continue;

			const actionType = entryType.action_type;
			if (actionType.action === "plan_presentation") {
				return actionType.plan;
			}
		}
		return null;
	}, [entries]);
}
