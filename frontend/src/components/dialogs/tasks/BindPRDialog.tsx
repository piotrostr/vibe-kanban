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
import { attemptsApi, repoApi } from "@/lib/api";
import { useTranslation } from "react-i18next";
import {
	Loader2,
	Link,
	Search,
	GitPullRequest,
	GitMerge,
	XCircle,
} from "lucide-react";
import NiceModal, { useModal } from "@ebay/nice-modal-react";
import { defineModal } from "@/lib/modals";
import type { PrListItem } from "shared/types";
import { cn } from "@/lib/utils";

interface BindPRDialogProps {
	attemptId: string;
	repoId: string;
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

const BindPRDialogImpl = NiceModal.create<BindPRDialogProps>(
	({ attemptId, repoId }) => {
		const modal = useModal();
		const { t } = useTranslation("tasks");
		const [searchQuery, setSearchQuery] = useState("");
		const [debouncedQuery, setDebouncedQuery] = useState("");
		const [recentPrs, setRecentPrs] = useState<PrListItem[]>([]);
		const [loading, setLoading] = useState(false);
		const [binding, setBinding] = useState(false);
		const [error, setError] = useState<string | null>(null);
		const [selectedPr, setSelectedPr] = useState<PrListItem | null>(null);
		const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

		// Fetch PRs on dialog open and when debounced query changes
		useEffect(() => {
			if (!modal.visible || !repoId) return;

			const fetchPrs = async () => {
				setLoading(true);
				setError(null);
				try {
					const response = await repoApi.listRecentPrs(repoId, {
						limit: 10,
						search: debouncedQuery || undefined,
					});
					setRecentPrs(response.prs);
				} catch (err) {
					console.error("Failed to fetch recent PRs:", err);
					setError(t("bindPrDialog.errors.failedToFetchPrs"));
				} finally {
					setLoading(false);
				}
			};

			fetchPrs();
		}, [modal.visible, repoId, debouncedQuery, t]);

		// Reset state when dialog opens
		useEffect(() => {
			if (modal.visible) {
				setSearchQuery("");
				setDebouncedQuery("");
				setSelectedPr(null);
				setError(null);
			}
		}, [modal.visible]);

		const handleConfirmBind = useCallback(async () => {
			if (!repoId || !attemptId || !selectedPr) return;

			setError(null);
			setBinding(true);

			const result = await attemptsApi.bindPR(attemptId, {
				repo_id: repoId,
				pr_number: selectedPr.number,
			});

			setBinding(false);

			if (result.success) {
				modal.resolve(result.data);
				modal.hide();
				return;
			}

			if (result.error) {
				switch (result.error.type) {
					case "pr_not_found_or_no_access":
						setError(
							t("bindPrDialog.errors.prNotFoundOrNoAccess", {
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
		}, [attemptId, repoId, selectedPr, modal, t]);

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

		return (
			<Dialog open={modal.visible} onOpenChange={() => handleCancel()}>
				<DialogContent className="sm:max-w-[500px]">
					<DialogHeader>
						<DialogTitle>{t("bindPrDialog.title")}</DialogTitle>
						<DialogDescription>
							{t("bindPrDialog.description")}
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4 py-4">
						<div className="relative">
							<Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
							<Input
								value={searchQuery}
								onChange={(e) => setSearchQuery(e.target.value)}
								placeholder={t("bindPrDialog.searchPlaceholder")}
								className="pl-9"
								autoFocus
							/>
							{isSearching && (
								<Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 h-4 w-4 animate-spin text-muted-foreground" />
							)}
						</div>

						<div className="max-h-[300px] overflow-y-auto rounded-md border">
							{loading && !isSearching ? (
								<div className="flex items-center justify-center p-8">
									<Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
								</div>
							) : recentPrs.length === 0 ? (
								<div className="p-4 text-center text-muted-foreground text-sm">
									{debouncedQuery
										? t("bindPrDialog.noSearchResults")
										: t("bindPrDialog.noPrsFound")}
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
													</div>
												</div>
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
						<Button
							onClick={handleConfirmBind}
							disabled={binding || !selectedPr}
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
