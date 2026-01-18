import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import { useCallback, useEffect, useMemo, useState } from "react";
import { tasksApi } from "@/lib/api";
import { useTranslation } from "react-i18next";
import {
	Loader2,
	FileText,
	GitBranch,
	Clock,
	ChevronRight,
	Search,
} from "lucide-react";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import type { SessionInfo } from "shared/types";
import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";

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

const ImportFromClaudeDialogImpl =
	NiceModal.create<ImportFromClaudeDialogProps>(
		({ projectId, projectPath }) => {
			const modal = useModal();
			const { t } = useTranslation(["tasks", "common"]);

			const [sessions, setSessions] = useState<SessionInfo[]>([]);
			const [searchQuery, setSearchQuery] = useState("");
			const [loading, setLoading] = useState(false);
			const [importing, setImporting] = useState(false);
			const [importingSessionId, setImportingSessionId] = useState<
				string | null
			>(null);
			const [error, setError] = useState<string | null>(null);

			// Filter sessions by search query
			const filteredSessions = useMemo(() => {
				if (!searchQuery.trim()) return sessions;
				const q = searchQuery.toLowerCase();
				return sessions.filter(
					(s) =>
						s.firstUserMessage?.toLowerCase().includes(q) ||
						s.gitBranch?.toLowerCase().includes(q) ||
						s.slug?.toLowerCase().includes(q) ||
						s.sessionId.toLowerCase().includes(q),
				);
			}, [sessions, searchQuery]);

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
					setSearchQuery("");
					setError(null);
					setImportingSessionId(null);
				}
			}, [modal.visible]);

			const handleSelectSession = useCallback(
				async (session: SessionInfo) => {
					setError(null);
					setImporting(true);
					setImportingSessionId(session.sessionId);

					try {
						const result = await tasksApi.importWithHistory(projectId, {
							sessionPath: session.path,
							taskTitle:
								session.firstUserMessage ?? session.slug ?? session.summary,
							defaultStatus: "todo",
						});
						modal.resolve(result);
						modal.hide();
					} catch (err) {
						console.error("Failed to import session:", err);
						setError("Failed to import session");
						setImporting(false);
						setImportingSessionId(null);
					}
				},
				[projectId, modal],
			);

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
								Select a Claude Code session to import with full conversation
								history
							</DialogDescription>
						</DialogHeader>

						<div className="space-y-4 py-4">
							{/* Search input */}
							<div className="relative">
								<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
								<Input
									type="text"
									placeholder="Search sessions..."
									value={searchQuery}
									onChange={(e) => setSearchQuery(e.target.value)}
									className="pl-9"
								/>
							</div>

							{/* Sessions list */}
							<div className="max-h-[350px] overflow-y-auto rounded-md border">
								{loading ? (
									<div className="flex items-center justify-center p-8">
										<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
									</div>
								) : filteredSessions.length === 0 ? (
									<div className="p-4 text-center text-muted-foreground text-sm">
										{sessions.length === 0
											? "No Claude Code sessions found"
											: "No sessions match your search"}
									</div>
								) : (
									<div className="divide-y">
										{filteredSessions.map((session) => (
											<button
												type="button"
												key={session.sessionId}
												onClick={() => handleSelectSession(session)}
												disabled={importing}
												className={cn(
													"w-full px-3 py-3 text-left hover:bg-muted/50 transition-colors disabled:opacity-50",
												)}
											>
												<div className="flex items-start gap-3">
													{importingSessionId === session.sessionId ? (
														<Loader2 className="h-4 w-4 mt-0.5 animate-spin text-muted-foreground" />
													) : (
														<FileText className="h-4 w-4 mt-0.5 text-muted-foreground" />
													)}
													<div className="flex-1 min-w-0">
														<div className="font-medium text-sm truncate">
															{session.firstUserMessage ||
																session.slug ||
																session.summary ||
																session.sessionId}
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

							{error && <Alert variant="destructive">{error}</Alert>}
						</div>

						<DialogFooter>
							<Button variant="outline" onClick={handleCancel}>
								{t("common:buttons.cancel")}
							</Button>
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
