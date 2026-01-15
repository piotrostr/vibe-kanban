import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Alert } from "@/components/ui/alert";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { repoApi, tasksApi } from "@/lib/api";
import { useTranslation } from "react-i18next";
import {
	Loader2,
	Search,
	GitPullRequest,
	GitMerge,
	XCircle,
	Download,
} from "lucide-react";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import { BaseCodingAgent } from "shared/types";
import type { PrListItem, ExecutorProfileId } from "shared/types";
import { cn } from "@/lib/utils";
import { ModeToggle } from "@/components/tasks/ModeToggle";
import { useNavigate } from "react-router-dom";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { useProjectRepos } from "@/hooks";

interface ImportPRAsTaskDialogProps {
	projectId: string;
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

function getPrStateIcon(state: string) {
	switch (state.toUpperCase()) {
		case "MERGED":
			return <GitMerge className="h-4 w-4 text-purple-500" />;
		case "CLOSED":
			return <XCircle className="h-4 w-4 text-red-500" />;
		default:
			return <GitPullRequest className="h-4 w-4 text-green-500" />;
	}
}

const ImportPRAsTaskDialogImpl = NiceModal.create<ImportPRAsTaskDialogProps>(
	({ projectId }) => {
		const modal = useModal();
		const { t } = useTranslation(["tasks", "common"]);
		const navigate = useNavigate();
		const { data: projectRepos = [] } = useProjectRepos(projectId);

		const [selectedRepoId, setSelectedRepoId] = useState<string | null>(null);
		const [searchQuery, setSearchQuery] = useState("");
		const [debouncedQuery, setDebouncedQuery] = useState("");
		const [recentPrs, setRecentPrs] = useState<PrListItem[]>([]);
		const [loading, setLoading] = useState(false);
		const [importing, setImporting] = useState(false);
		const [error, setError] = useState<string | null>(null);
		const [selectedPr, setSelectedPr] = useState<PrListItem | null>(null);
		const [isPlanMode, setIsPlanMode] = useState(false);
		const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

		// Set default repo when data loads
		useEffect(() => {
			if (projectRepos.length > 0 && !selectedRepoId) {
				setSelectedRepoId(projectRepos[0].id);
			}
		}, [projectRepos, selectedRepoId]);

		// Debounce search query
		useEffect(() => {
			if (debounceTimerRef.current) {
				clearTimeout(debounceTimerRef.current);
			}
			debounceTimerRef.current = setTimeout(() => {
				setDebouncedQuery(searchQuery);
			}, 300);

			return () => {
				if (debounceTimerRef.current) {
					clearTimeout(debounceTimerRef.current);
				}
			};
		}, [searchQuery]);

		// Fetch PRs when dialog opens and when query/repo changes
		useEffect(() => {
			if (!modal.visible || !selectedRepoId) return;

			const fetchPrs = async () => {
				setLoading(true);
				setError(null);
				try {
					const response = await repoApi.listRecentPrs(selectedRepoId, {
						limit: 10,
						search: debouncedQuery || undefined,
					});
					setRecentPrs(response.prs);
				} catch (err) {
					console.error("Failed to fetch recent PRs:", err);
					setError(t("importPrDialog.errors.failedToFetchPrs"));
				} finally {
					setLoading(false);
				}
			};

			fetchPrs();
		}, [modal.visible, selectedRepoId, debouncedQuery, t]);

		// Reset state when dialog opens
		useEffect(() => {
			if (modal.visible) {
				setSearchQuery("");
				setDebouncedQuery("");
				setSelectedPr(null);
				setError(null);
			}
		}, [modal.visible]);

		const handleConfirmImport = useCallback(async () => {
			if (!selectedRepoId || !selectedPr) return;

			const executorProfileId: ExecutorProfileId = {
				executor: BaseCodingAgent.CLAUDE_CODE,
				variant: isPlanMode ? "PLAN" : null,
			};

			setError(null);
			setImporting(true);

			try {
				const result = await tasksApi.importFromPr({
					projectId,
					repoId: selectedRepoId,
					prNumber: selectedPr.number,
					executorProfileId,
				});

				if (result.success) {
					modal.resolve(result.data);
					modal.hide();
					navigate(
						`/projects/${projectId}/tasks/${result.data.id}/attempts/latest`,
					);
					return;
				}

				if (!result.success && result.error) {
					switch (result.error.type) {
						case "pr_not_found_or_no_access":
							setError(
								t("importPrDialog.errors.prNotFoundOrNoAccess", {
									number: Number(result.error.pr_number),
								}),
							);
							break;
						case "github_cli_not_installed":
							setError(t("importPrDialog.errors.githubCliNotInstalled"));
							break;
						case "github_cli_not_logged_in":
							setError(t("importPrDialog.errors.githubCliNotLoggedIn"));
							break;
						default:
							setError(
								result.message || t("importPrDialog.errors.failedToImport"),
							);
					}
				} else if (!result.success) {
					setError(result.message || t("importPrDialog.errors.failedToImport"));
				}
			} catch (err) {
				console.error("Failed to import PR as task:", err);
				setError(t("importPrDialog.errors.failedToImport"));
			} finally {
				setImporting(false);
			}
		}, [projectId, selectedRepoId, selectedPr, isPlanMode, modal, navigate, t]);

		const handleCancel = useCallback(() => {
			modal.reject("canceled");
			modal.hide();
		}, [modal]);

		const handleSelectPr = useCallback((pr: PrListItem) => {
			setSelectedPr(pr);
			setError(null);
		}, []);

		const isSearching = useMemo(
			() => searchQuery !== debouncedQuery,
			[searchQuery, debouncedQuery],
		);

		const canImport = selectedRepoId && selectedPr;

		return (
			<Dialog open={modal.visible} onOpenChange={() => handleCancel()}>
				<DialogContent className="sm:max-w-[550px]">
					<DialogHeader>
						<DialogTitle className="flex items-center gap-2">
							<GitPullRequest className="h-5 w-5" />
							{t("importPrDialog.title")}
						</DialogTitle>
						<DialogDescription>
							{t("importPrDialog.description")}
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4 py-4">
						{projectRepos.length > 1 && (
							<div className="flex items-center gap-2">
								<span className="text-sm text-muted-foreground min-w-[80px]">
									{t("importPrDialog.repository")}
								</span>
								<Select
									value={selectedRepoId || ""}
									onValueChange={setSelectedRepoId}
								>
									<SelectTrigger className="flex-1">
										<SelectValue
											placeholder={t("importPrDialog.selectRepository")}
										/>
									</SelectTrigger>
									<SelectContent>
										{projectRepos.map((repo) => (
											<SelectItem key={repo.id} value={repo.id}>
												{repo.name}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
						)}

						<div className="flex items-center gap-2">
							<span className="text-sm text-muted-foreground min-w-[80px]">
								{t("importPrDialog.executor")}
							</span>
							<ModeToggle
								isPlanMode={isPlanMode}
								onToggle={() => setIsPlanMode((prev) => !prev)}
								disabled={importing}
							/>
						</div>

						<div className="relative">
							<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
							<Input
								value={searchQuery}
								onChange={(e) => setSearchQuery(e.target.value)}
								placeholder={t("importPrDialog.searchPlaceholder")}
								className="pl-9"
								autoFocus
							/>
							{isSearching && (
								<Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 animate-spin text-muted-foreground" />
							)}
						</div>

						<div className="max-h-[280px] overflow-y-auto rounded-md border">
							{loading && !isSearching ? (
								<div className="flex items-center justify-center p-8">
									<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
								</div>
							) : recentPrs.length === 0 ? (
								<div className="p-4 text-center text-muted-foreground text-sm">
									{debouncedQuery
										? t("importPrDialog.noSearchResults")
										: t("importPrDialog.noPrsFound")}
								</div>
							) : (
								<div className="divide-y">
									{recentPrs.map((pr) => (
										<button
											type="button"
											key={pr.number.toString()}
											onClick={() => handleSelectPr(pr)}
											className={cn(
												"w-full px-3 py-2 text-left hover:bg-muted/50 transition-colors",
												selectedPr?.number === pr.number && "bg-muted",
											)}
										>
											<div className="flex items-start gap-2">
												<div className="mt-0.5">{getPrStateIcon(pr.state)}</div>
												<div className="flex-1 min-w-0">
													<div className="flex items-center gap-2">
														<span className="font-medium text-muted-foreground text-sm">
															#{pr.number.toString()}
														</span>
														<span className="truncate text-sm">{pr.title}</span>
													</div>
													<div className="flex items-center gap-2 text-xs text-muted-foreground mt-0.5">
														<span>{pr.author.login}</span>
														<span>-</span>
														<span>{formatRelativeTime(pr.createdAt)}</span>
														<span>-</span>
														<span className="font-mono text-xs">
															{pr.headRefName}
														</span>
													</div>
												</div>
											</div>
										</button>
									))}
								</div>
							)}
						</div>

						{selectedPr && selectedPr.body && (
							<div className="rounded-md border p-3 bg-muted/30">
								<div className="text-xs text-muted-foreground mb-1">
									{t("importPrDialog.prDescription")}
								</div>
								<div className="text-sm line-clamp-3 whitespace-pre-wrap">
									{selectedPr.body}
								</div>
							</div>
						)}

						{error && <Alert variant="destructive">{error}</Alert>}
					</div>
					<DialogFooter>
						<Button variant="outline" onClick={handleCancel}>
							{t("common:buttons.cancel")}
						</Button>
						<Button
							onClick={handleConfirmImport}
							disabled={importing || !canImport}
							className="bg-blue-600 hover:bg-blue-700"
						>
							{importing ? (
								<>
									<Loader2 className="mr-2 h-4 w-4 animate-spin" />
									{t("importPrDialog.importing")}
								</>
							) : (
								<>
									<Download className="mr-2 h-4 w-4" />
									{t("importPrDialog.importButton")}
								</>
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		);
	},
);

export const ImportPRAsTaskDialog = defineModal<
	ImportPRAsTaskDialogProps,
	void
>(ImportPRAsTaskDialogImpl);
