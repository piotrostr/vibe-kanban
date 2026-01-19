import { Link, useLocation, useSearchParams } from "react-router-dom";
import { useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	FolderOpen,
	Layers,
	Settings,
	BookOpen,
	MessageCircleQuestion,
	Menu,
	Plus,
	LogOut,
	LogIn,
	GitPullRequest,
	Eye,
	EyeOff,
} from "lucide-react";
import { Logo } from "@/components/Logo";
import { SearchBar } from "@/components/SearchBar";
import { useSearch } from "@/contexts/SearchContext";
import { openTaskForm } from "@/lib/openTaskForm";
import { useProject } from "@/contexts/ProjectContext";
import { useOpenProjectInEditor } from "@/hooks/useOpenProjectInEditor";
import { OpenInIdeButton } from "@/components/ide/OpenInIdeButton";
import { useProjectRepos } from "@/hooks";
import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { OAuthDialog } from "@/components/dialogs/global/OAuthDialog";
import {
	OnboardingDialog,
	type OnboardingResult,
} from "@/components/dialogs/global/OnboardingDialog";
import { useUserSystem } from "@/components/ConfigProvider";
import { oauthApi } from "@/lib/api";
import { HelpCircle } from "lucide-react";
import { ImportPRAsTaskDialog } from "@/components/dialogs/tasks/ImportPRAsTaskDialog";
import { usePrivacy } from "@/contexts/PrivacyContext";
import { openExternal } from "@/lib/openExternal";

const INTERNAL_NAV = [
	{ label: "Projects", icon: FolderOpen, to: "/settings/projects" },
	{ label: "All Tasks", icon: Layers, to: "/" },
];

const EXTERNAL_LINKS = [
	{
		label: "Docs",
		icon: BookOpen,
		href: "https://vibekanban.com/docs",
	},
	{
		label: "Support",
		icon: MessageCircleQuestion,
		href: "https://github.com/BloopAI/vibe-kanban/issues",
	},
];

function NavDivider() {
	return (
		<div
			className="mx-2 h-6 w-px bg-border/60"
			role="separator"
			aria-orientation="vertical"
		/>
	);
}

export function Navbar() {
	const location = useLocation();
	const [searchParams, setSearchParams] = useSearchParams();
	const { projectId, project } = useProject();
	const { query, setQuery, active, clear, registerInputRef } = useSearch();
	const handleOpenInEditor = useOpenProjectInEditor(project || null);
	const { loginStatus, reloadSystem, updateAndSaveConfig } = useUserSystem();
	const { privacyMode, togglePrivacy } = usePrivacy();

	const { data: repos } = useProjectRepos(projectId);
	const isSingleRepoProject = repos?.length === 1;

	const setSearchBarRef = useCallback(
		(node: HTMLInputElement | null) => {
			registerInputRef(node);
		},
		[registerInputRef],
	);
	const { t } = useTranslation(["tasks", "common"]);
	// Navbar is global, but the share tasks toggle only makes sense on the tasks route
	const isTasksRoute = /^\/projects\/[^/]+\/tasks/.test(location.pathname);
	const showSharedTasks = searchParams.get("shared") !== "off";
	const shouldShowSharedToggle =
		isTasksRoute && active && project?.remote_project_id != null;

	const handleSharedToggle = useCallback(
		(checked: boolean) => {
			const params = new URLSearchParams(searchParams);
			if (checked) {
				params.delete("shared");
			} else {
				params.set("shared", "off");
			}
			setSearchParams(params, { replace: true });
		},
		[searchParams, setSearchParams],
	);

	const handleCreateTask = () => {
		if (projectId) {
			openTaskForm({ mode: "create", projectId });
		}
	};

	const handleImportFromPR = () => {
		if (projectId) {
			ImportPRAsTaskDialog.show({ projectId });
		}
	};

	const handleOpenInIDE = () => {
		handleOpenInEditor();
	};

	const handleOpenOAuth = async () => {
		const profile = await OAuthDialog.show();
		if (profile) {
			await reloadSystem();
		}
	};

	const handleOAuthLogout = async () => {
		try {
			await oauthApi.logout();
			await reloadSystem();
		} catch (err) {
			console.error("Error logging out:", err);
		}
	};

	const handleOpenOnboarding = async () => {
		const result: OnboardingResult | undefined = await OnboardingDialog.show();
		if (result) {
			await updateAndSaveConfig({
				executor_profile: result.profile,
				editor: result.editor,
			});
		}
		OnboardingDialog.hide();
	};

	const isOAuthLoggedIn = loginStatus?.status === "loggedin";

	return (
		<div className="border-b bg-background">
			<div className="w-full px-3">
				<div className="flex items-center h-12 py-2">
					<div className="flex-1 flex items-center">
						<Link to="/" className="flex items-center gap-2">
							<Logo />
							<span className="font-semibold text-lg">Vibe</span>
						</Link>
					</div>

					<div className="hidden sm:flex items-center gap-2">
						<SearchBar
							ref={setSearchBarRef}
							className="shrink-0"
							value={query}
							onChange={setQuery}
							disabled={!active}
							onClear={clear}
							project={project || null}
						/>
					</div>

					<div className="flex flex-1 items-center justify-end gap-1">
						{isOAuthLoggedIn && shouldShowSharedToggle ? (
							<>
								<div className="flex items-center gap-4">
									<TooltipProvider>
										<Tooltip>
											<TooltipTrigger asChild>
												<div>
													<Switch
														checked={showSharedTasks}
														onCheckedChange={handleSharedToggle}
														aria-label={t("tasks:filters.sharedToggleAria")}
													/>
												</div>
											</TooltipTrigger>
											<TooltipContent side="bottom">
												{t("tasks:filters.sharedToggleTooltip")}
											</TooltipContent>
										</Tooltip>
									</TooltipProvider>
								</div>
								<NavDivider />
							</>
						) : null}

						{projectId ? (
							<>
								<div className="flex items-center gap-1">
									{isSingleRepoProject && (
										<OpenInIdeButton
											onClick={handleOpenInIDE}
											className="h-9 w-9"
										/>
									)}
									<DropdownMenu>
										<DropdownMenuTrigger asChild>
											<Button
												variant="ghost"
												size="icon"
												className="h-9 w-9"
												aria-label="Create new task"
											>
												<Plus className="h-4 w-4" />
											</Button>
										</DropdownMenuTrigger>
										<DropdownMenuContent align="end">
											<DropdownMenuItem onSelect={handleCreateTask}>
												<Plus className="h-4 w-4 mr-2" />
												{t("tasks:actions.newTask")}
											</DropdownMenuItem>
											<DropdownMenuItem onSelect={handleImportFromPR}>
												<GitPullRequest className="h-4 w-4 mr-2" />
												{t("tasks:actions.importFromPr")}
											</DropdownMenuItem>
										</DropdownMenuContent>
									</DropdownMenu>
								</div>
								<NavDivider />
							</>
						) : null}

						<div className="flex items-center gap-1">
							<TooltipProvider>
								<Tooltip>
									<TooltipTrigger asChild>
										<Button
											variant="ghost"
											size="icon"
											className="h-9 w-9"
											onClick={togglePrivacy}
											aria-label="Toggle privacy mode"
										>
											{privacyMode ? (
												<EyeOff className="h-4 w-4" />
											) : (
												<Eye className="h-4 w-4" />
											)}
										</Button>
									</TooltipTrigger>
									<TooltipContent side="bottom">
										{privacyMode ? "Show content" : "Hide content"}
									</TooltipContent>
								</Tooltip>
							</TooltipProvider>

							<Button
								variant="ghost"
								size="icon"
								className="h-9 w-9"
								asChild
								aria-label="Settings"
							>
								<Link
									to={
										projectId
											? `/settings/projects?projectId=${projectId}`
											: "/settings"
									}
								>
									<Settings className="h-4 w-4" />
								</Link>
							</Button>

							<DropdownMenu>
								<DropdownMenuTrigger asChild>
									<Button
										variant="ghost"
										size="icon"
										className="h-9 w-9"
										aria-label="Main navigation"
									>
										<Menu className="h-4 w-4" />
									</Button>
								</DropdownMenuTrigger>

								<DropdownMenuContent align="end">
									{INTERNAL_NAV.map((item) => {
										const active = location.pathname.startsWith(item.to);
										const Icon = item.icon;
										return (
											<DropdownMenuItem
												key={item.to}
												asChild
												className={active ? "bg-accent" : ""}
											>
												<Link to={item.to}>
													<Icon className="mr-2 h-4 w-4" />
													{item.label}
												</Link>
											</DropdownMenuItem>
										);
									})}

									<DropdownMenuSeparator />

									{EXTERNAL_LINKS.map((item) => {
										const Icon = item.icon;
										return (
											<DropdownMenuItem
												key={item.href}
												onSelect={() => void openExternal(item.href)}
											>
												<Icon className="mr-2 h-4 w-4" />
												{item.label}
											</DropdownMenuItem>
										);
									})}

									<DropdownMenuItem onSelect={handleOpenOnboarding}>
										<HelpCircle className="mr-2 h-4 w-4" />
										Setup
									</DropdownMenuItem>

									<DropdownMenuSeparator />

									{isOAuthLoggedIn ? (
										<DropdownMenuItem onSelect={handleOAuthLogout}>
											<LogOut className="mr-2 h-4 w-4" />
											{t("common:signOut")}
										</DropdownMenuItem>
									) : (
										<DropdownMenuItem onSelect={handleOpenOAuth}>
											<LogIn className="mr-2 h-4 w-4" />
											Sign in
										</DropdownMenuItem>
									)}
								</DropdownMenuContent>
							</DropdownMenu>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
