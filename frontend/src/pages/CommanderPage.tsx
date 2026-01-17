import { useParams, Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, Loader2, Send, AlertCircle } from "lucide-react";
import { useState, useCallback } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { commanderApi } from "@/lib/api";
import { ClaudeCommanderIcon } from "@/components/icons/ClaudeCommanderIcon";
import WYSIWYGEditor from "@/components/ui/wysiwyg";
import { useProject } from "@/contexts/ProjectContext";

export function CommanderPage() {
	const { projectId = "" } = useParams<{ projectId: string }>();
	const { project } = useProject();
	const [prompt, setPrompt] = useState("");
	const [isSubmitting, setIsSubmitting] = useState(false);

	const {
		data: commanderSession,
		isLoading,
		error,
	} = useQuery({
		queryKey: ["commander", projectId],
		queryFn: () => commanderApi.getOrCreate(projectId),
		enabled: !!projectId,
	});

	const {
		data: processes,
		isLoading: processesLoading,
		refetch: refetchProcesses,
	} = useQuery({
		queryKey: ["commander-processes", commanderSession?.id],
		queryFn: () => commanderApi.getProcesses(commanderSession!.id),
		enabled: !!commanderSession?.id,
	});

	const handleSubmit = useCallback(async () => {
		if (!commanderSession || !prompt.trim() || isSubmitting) return;

		setIsSubmitting(true);
		try {
			await commanderApi.followUp(commanderSession.id, {
				prompt,
				variant: null,
			});
			setPrompt("");
			refetchProcesses();
		} catch (err) {
			console.error("Failed to send message:", err);
		} finally {
			setIsSubmitting(false);
		}
	}, [commanderSession, prompt, isSubmitting, refetchProcesses]);

	if (isLoading) {
		return (
			<div className="h-screen flex items-center justify-center">
				<Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
			</div>
		);
	}

	if (error) {
		return (
			<div className="h-screen flex items-center justify-center">
				<Card className="p-6 max-w-md">
					<div className="flex items-center gap-2 text-destructive mb-4">
						<AlertCircle className="h-5 w-5" />
						<span className="font-medium">Failed to load commander</span>
					</div>
					<p className="text-sm text-muted-foreground">
						{error instanceof Error ? error.message : "Unknown error"}
					</p>
					<Button asChild className="mt-4">
						<Link to={`/projects/${projectId}/tasks`}>Back to Tasks</Link>
					</Button>
				</Card>
			</div>
		);
	}

	return (
		<div className="h-screen flex flex-col bg-background">
			{/* Header */}
			<header className="border-b px-4 py-3 flex items-center gap-3">
				<Button variant="ghost" size="icon" asChild>
					<Link to={`/projects/${projectId}/tasks`}>
						<ArrowLeft className="h-4 w-4" />
					</Link>
				</Button>
				<ClaudeCommanderIcon className="h-6 w-6" />
				<div className="flex-1">
					<h1 className="font-semibold">Commander</h1>
					<p className="text-xs text-muted-foreground">
						{project?.name || "Project"}
					</p>
				</div>
			</header>

			{/* Chat area */}
			<main className="flex-1 min-h-0 overflow-auto p-4">
				{processesLoading ? (
					<div className="flex items-center justify-center h-full">
						<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
					</div>
				) : processes && processes.length > 0 ? (
					<div className="space-y-4 max-w-3xl mx-auto">
						{processes.map((process) => (
							<Card key={process.id} className="p-4">
								<div className="text-sm">
									<span className="font-medium">
										Process {process.id.slice(0, 8)}
									</span>
									<span className="ml-2 text-muted-foreground">
										Status: {process.status}
									</span>
								</div>
							</Card>
						))}
					</div>
				) : (
					<div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground">
						<ClaudeCommanderIcon className="h-16 w-16 mb-4 opacity-50" />
						<p className="text-lg font-medium mb-2">No conversations yet</p>
						<p className="text-sm max-w-md">
							Start a conversation with the Commander to manage your tasks and
							prototype features in a persistent worktree.
						</p>
					</div>
				)}
			</main>

			{/* Input area */}
			<footer className="border-t p-4">
				<div className="max-w-3xl mx-auto">
					<div className="flex gap-2">
						<div className="flex-1 min-h-[80px] border rounded-md bg-background">
							<WYSIWYGEditor
								value={prompt}
								onChange={setPrompt}
								placeholder="Ask the Commander..."
								disabled={isSubmitting}
							/>
						</div>
						<Button
							onClick={handleSubmit}
							disabled={!prompt.trim() || isSubmitting}
							className="h-[80px]"
						>
							{isSubmitting ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : (
								<Send className="h-4 w-4" />
							)}
						</Button>
					</div>
					<p className="text-xs text-muted-foreground mt-2 text-center">
						Commander has access to the vibe-kanban MCP for task management
					</p>
				</div>
			</footer>
		</div>
	);
}
