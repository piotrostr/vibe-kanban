import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Alert } from "@/components/ui/alert";
import { useCallback, useEffect, useState } from "react";
import { tasksApi } from "@/lib/api";
import { useTranslation } from "react-i18next";
import {
	Loader2,
	Download,
	FileText,
	GitBranch,
	Clock,
	ChevronRight,
} from "lucide-react";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import type { ExtractedTask, SessionInfo } from "shared/types";
import { cn } from "@/lib/utils";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";

interface ImportFromClaudeDialogProps {
	projectId: string;
	projectPath?: string;
}

function formatRelativeTime(dateStr: string): string {
	const date = new Date(dateStr);
	const now = Date.now();
	const diffMs = now - date.getTime();
	const diffSec = Math.floor(diffMs / 1000);
	const diffMin = Math.floor(diffSec / 60);
	const diffHour = Math.floor(diffMin / 60);
	const diffDay = Math.floor(diffHour / 24);

	if (diffDay > 0) return `${diffDay}d ago`;
	if (diffHour > 0) return `${diffHour}h ago`;
	if (diffMin > 0) return `${diffMin}m ago`;
	return "just now";
}

type Step = "sessions" | "preview";

const ImportFromClaudeDialogImpl =
	NiceModal.create<ImportFromClaudeDialogProps>(
		({ projectId, projectPath }) => {
			const modal = useModal();
			const { t } = useTranslation(["tasks", "common"]);

			const [step, setStep] = useState<Step>("sessions");
			const [sessions, setSessions] = useState<SessionInfo[]>([]);
			const [selectedSession, setSelectedSession] =
				useState<SessionInfo | null>(null);
			const [extractedTasks, setExtractedTasks] = useState<ExtractedTask[]>([]);
			const [selectedTaskIds, setSelectedTaskIds] = useState<Set<string>>(
				new Set(),
			);
			const [defaultStatus, setDefaultStatus] = useState<string>("backlog");
			const [loading, setLoading] = useState(false);
			const [importing, setImporting] = useState(false);
			const [error, setError] = useState<string | null>(null);

			// Fetch available sessions when dialog opens
			useEffect(() => {
				if (!modal.visible) return;

				const fetchSessions = async () => {
					setLoading(true);
					setError(null);
					try {
						const response = await tasksApi.listClaudeSessions(projectPath);
						setSessions(response.sessions);
					} catch (err) {
						console.error("Failed to fetch Claude sessions:", err);
						setError("Failed to load Claude Code sessions");
					} finally {
						setLoading(false);
					}
				};

				fetchSessions();
			}, [modal.visible, projectPath]);

			// Reset state when dialog opens
			useEffect(() => {
				if (modal.visible) {
					setStep("sessions");
					setSelectedSession(null);
					setExtractedTasks([]);
					setSelectedTaskIds(new Set());
					setError(null);
				}
			}, [modal.visible]);

			const handleSelectSession = useCallback(async (session: SessionInfo) => {
				setSelectedSession(session);
				setError(null);
				setLoading(true);

				try {
					const response = await tasksApi.previewClaudeSession({
						sessionPath: session.path,
					});
					setExtractedTasks(response.items);
					setSelectedTaskIds(new Set(response.items.map((item) => item.id)));
					setStep("preview");
				} catch (err) {
					console.error("Failed to preview session:", err);
					setError("Failed to parse session file");
				} finally {
					setLoading(false);
				}
			}, []);

			const handleToggleTask = useCallback((taskId: string) => {
				setSelectedTaskIds((prev) => {
					const next = new Set(prev);
					if (next.has(taskId)) {
						next.delete(taskId);
					} else {
						next.add(taskId);
					}
					return next;
				});
			}, []);

			const handleSelectAll = useCallback(() => {
				setSelectedTaskIds(new Set(extractedTasks.map((t) => t.id)));
			}, [extractedTasks]);

			const handleSelectNone = useCallback(() => {
				setSelectedTaskIds(new Set());
			}, []);

			const handleBack = useCallback(() => {
				setStep("sessions");
				setSelectedSession(null);
				setExtractedTasks([]);
				setSelectedTaskIds(new Set());
				setError(null);
			}, []);

			const handleImport = useCallback(async () => {
				if (!selectedSession || selectedTaskIds.size === 0) return;

				setError(null);
				setImporting(true);

				try {
					const result = await tasksApi.importFromClaudeSession(projectId, {
						sessionPath: selectedSession.path,
						selectedItemIds: Array.from(selectedTaskIds),
						defaultStatus,
					});

					if (result.errors.length > 0) {
						console.warn("Import errors:", result.errors);
					}

					modal.resolve(result);
					modal.hide();
				} catch (err) {
					console.error("Failed to import tasks:", err);
					setError("Failed to import tasks from session");
				} finally {
					setImporting(false);
				}
			}, [projectId, selectedSession, selectedTaskIds, defaultStatus, modal]);

			const handleCancel = useCallback(() => {
				modal.reject("canceled");
				modal.hide();
			}, [modal]);

			return (
				<Dialog open={modal.visible} onOpenChange={() => handleCancel()}>
					<DialogContent className="sm:max-w-[600px]">
						<DialogHeader>
							<DialogTitle className="flex items-center gap-2">
								<FileText className="h-5 w-5" />
								Import from Claude Code
							</DialogTitle>
							<DialogDescription>
								{step === "sessions"
									? "Select a Claude Code session to import tasks from"
									: "Select which work items to import as tasks"}
							</DialogDescription>
						</DialogHeader>

						<div className="space-y-4 py-4">
							{step === "sessions" && (
								<div className="max-h-[350px] overflow-y-auto rounded-md border">
									{loading ? (
										<div className="flex items-center justify-center p-8">
											<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
										</div>
									) : sessions.length === 0 ? (
										<div className="p-4 text-center text-muted-foreground text-sm">
											No Claude Code sessions found
										</div>
									) : (
										<div className="divide-y">
											{sessions.map((session) => (
												<button
													type="button"
													key={session.sessionId}
													onClick={() => handleSelectSession(session)}
													className={cn(
														"w-full px-3 py-3 text-left hover:bg-muted/50 transition-colors",
														selectedSession?.sessionId === session.sessionId &&
															"bg-muted",
													)}
												>
													<div className="flex items-start gap-3">
														<FileText className="h-4 w-4 mt-0.5 text-muted-foreground" />
														<div className="flex-1 min-w-0">
															<div className="font-medium text-sm truncate">
																{session.summary || session.sessionId}
															</div>
															<div className="flex items-center gap-3 text-xs text-muted-foreground mt-1">
																<span className="flex items-center gap-1">
																	<Clock className="h-3 w-3" />
																	{formatRelativeTime(session.lastModified)}
																</span>
																{session.gitBranch && (
																	<span className="flex items-center gap-1">
																		<GitBranch className="h-3 w-3" />
																		{session.gitBranch}
																	</span>
																)}
																<span>{session.messageCount} messages</span>
															</div>
														</div>
														<ChevronRight className="h-4 w-4 text-muted-foreground" />
													</div>
												</button>
											))}
										</div>
									)}
								</div>
							)}

							{step === "preview" && (
								<>
									<div className="flex items-center justify-between">
										<div className="flex gap-2">
											<Button
												variant="ghost"
												size="sm"
												className="h-7 px-2 text-xs"
												onClick={handleSelectAll}
											>
												Select All
											</Button>
											<Button
												variant="ghost"
												size="sm"
												className="h-7 px-2 text-xs"
												onClick={handleSelectNone}
											>
												Select None
											</Button>
										</div>
										<div className="flex items-center gap-2">
											<span className="text-sm text-muted-foreground">
												Default status:
											</span>
											<Select
												value={defaultStatus}
												onValueChange={setDefaultStatus}
											>
												<SelectTrigger className="w-28 h-7">
													<SelectValue />
												</SelectTrigger>
												<SelectContent>
													<SelectItem value="backlog">Backlog</SelectItem>
													<SelectItem value="todo">Todo</SelectItem>
													<SelectItem value="inprogress">
														In Progress
													</SelectItem>
												</SelectContent>
											</Select>
										</div>
									</div>

									<div className="max-h-[280px] overflow-y-auto rounded-md border">
										{extractedTasks.length === 0 ? (
											<div className="p-4 text-center text-muted-foreground text-sm">
												No importable tasks found in this session
											</div>
										) : (
											<div className="divide-y">
												{extractedTasks.map((task) => (
													<label
														key={task.id}
														className={cn(
															"flex items-start gap-3 px-3 py-2 cursor-pointer hover:bg-muted/50 transition-colors",
															selectedTaskIds.has(task.id) && "bg-muted/30",
														)}
													>
														<Checkbox
															checked={selectedTaskIds.has(task.id)}
															onCheckedChange={() => handleToggleTask(task.id)}
															className="mt-0.5"
														/>
														<div className="flex-1 min-w-0">
															<div className="text-sm font-medium truncate">
																{task.title}
															</div>
															{task.description &&
																task.description !== task.title && (
																	<div className="text-xs text-muted-foreground line-clamp-2 mt-0.5">
																		{task.description}
																	</div>
																)}
															<div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
																{task.timestamp && (
																	<span>
																		{formatRelativeTime(task.timestamp)}
																	</span>
																)}
																{task.branch && (
																	<span className="flex items-center gap-1">
																		<GitBranch className="h-3 w-3" />
																		{task.branch}
																	</span>
																)}
															</div>
														</div>
													</label>
												))}
											</div>
										)}
									</div>
								</>
							)}

							{error && <Alert variant="destructive">{error}</Alert>}
						</div>

						<DialogFooter>
							{step === "preview" && (
								<Button
									variant="outline"
									onClick={handleBack}
									className="mr-auto"
								>
									Back
								</Button>
							)}
							<Button variant="outline" onClick={handleCancel}>
								{t("common:buttons.cancel")}
							</Button>
							{step === "preview" && (
								<Button
									onClick={handleImport}
									disabled={importing || selectedTaskIds.size === 0}
									className="bg-blue-600 hover:bg-blue-700"
								>
									{importing ? (
										<>
											<Loader2 className="mr-2 h-4 w-4 animate-spin" />
											Importing...
										</>
									) : (
										<>
											<Download className="mr-2 h-4 w-4" />
											Import {selectedTaskIds.size} Task
											{selectedTaskIds.size !== 1 ? "s" : ""}
										</>
									)}
								</Button>
							)}
						</DialogFooter>
					</DialogContent>
				</Dialog>
			);
		},
	);

export const ImportFromClaudeDialog = defineModal<
	ImportFromClaudeDialogProps,
	void
>(ImportFromClaudeDialogImpl);
