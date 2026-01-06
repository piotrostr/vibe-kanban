import {
	createContext,
	useContext,
	ReactNode,
	useMemo,
	useEffect,
} from "react";
import { useLocation } from "react-router-dom";
import type { Project } from "shared/types";
import { useProjects } from "@/hooks/useProjects";

interface ProjectContextValue {
	projectId: string | undefined;
	project: Project | undefined;
	isLoading: boolean;
	error: Error | null;
	isError: boolean;
}

const ProjectContext = createContext<ProjectContextValue | null>(null);

interface ProjectProviderProps {
	children: ReactNode;
	/** Optional override for projectId - if not provided, extracts from URL */
	projectId?: string;
}

export function ProjectProvider({
	children,
	projectId: projectIdProp,
}: ProjectProviderProps) {
	const location = useLocation();

	// Extract projectId from current route path, or use prop override
	const projectId = useMemo(() => {
		if (projectIdProp) return projectIdProp;
		const match = location.pathname.match(/^\/projects\/([^/]+)/);
		return match ? match[1] : undefined;
	}, [location.pathname, projectIdProp]);

	const { projectsById, isLoading, error } = useProjects();
	const project = projectId ? projectsById[projectId] : undefined;

	const value = useMemo(
		() => ({
			projectId,
			project,
			isLoading,
			error,
			isError: !!error,
		}),
		[projectId, project, isLoading, error],
	);

	// Centralized page title management
	useEffect(() => {
		if (project) {
			document.title = `${project.name} | Vibe`;
		} else {
			document.title = "Vibe";
		}
	}, [project]);

	return (
		<ProjectContext.Provider value={value}>{children}</ProjectContext.Provider>
	);
}

export function useProject(): ProjectContextValue {
	const context = useContext(ProjectContext);
	if (!context) {
		throw new Error("useProject must be used within a ProjectProvider");
	}
	return context;
}
