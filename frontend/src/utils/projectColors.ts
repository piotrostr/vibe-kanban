// Predefined color palette for projects
const PROJECT_COLORS = [
	"#EF4444", // red
	"#F97316", // orange
	"#EAB308", // yellow
	"#22C55E", // green
	"#06B6D4", // cyan
	"#3B82F6", // blue
	"#8B5CF6", // violet
	"#EC4899", // pink
	"#14B8A6", // teal
	"#6366F1", // indigo
];

/**
 * Get a deterministic color for a project based on its ID.
 * Uses a hash of the project ID to assign a consistent color from the palette.
 */
export function getProjectColor(
	projectId: string,
	customColor?: string | null,
): string {
	if (customColor) return customColor;

	// Hash the project ID to get a consistent index
	let hash = 0;
	for (let i = 0; i < projectId.length; i++) {
		hash = (hash << 5) - hash + projectId.charCodeAt(i);
		hash = hash & hash; // Convert to 32-bit integer
	}

	return PROJECT_COLORS[Math.abs(hash) % PROJECT_COLORS.length];
}
