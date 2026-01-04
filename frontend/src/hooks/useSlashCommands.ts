import { useQuery } from "@tanstack/react-query";
import { slashCommandsApi } from "@/lib/api";
import type { SlashCommand } from "shared/types";

export function useSlashCommands() {
	const { data, isLoading, isError, error } = useQuery({
		queryKey: ["slash-commands"],
		queryFn: () => slashCommandsApi.list(),
		staleTime: 1000 * 60, // 1 minute cache
	});

	return {
		commands: data ?? [],
		isLoading,
		isError,
		error,
	};
}

export type { SlashCommand };
