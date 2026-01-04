import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { ChevronDown, ChevronRight } from "lucide-react";

type JsonObject = Record<string, unknown>;

interface JsonMessageRendererProps {
	content: string;
}

// Fields to display as badges (type indicators)
const BADGE_FIELDS = ["type", "subtype", "status"];

// Fields to display prominently as main text
const TEXT_FIELDS = ["message", "result", "content", "error", "output"];

// Fields to hide in collapsed sections (verbose arrays)
const COLLAPSIBLE_FIELDS = [
	"tools",
	"mcp_servers",
	"plugins",
	"agents",
	"skills",
	"slash_commands",
	"modelUsage",
	"usage",
	"permission_denials",
];

// Fields to skip entirely (internal/noisy)
const SKIP_FIELDS = ["uuid", "is_error"];

function isJsonObject(value: unknown): value is JsonObject {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function CollapsibleSection({
	label,
	data,
	itemCount,
}: {
	label: string;
	data: unknown;
	itemCount?: number;
}) {
	const [isExpanded, setIsExpanded] = useState(false);

	return (
		<div className="border-t border-border/50 pt-2 mt-2">
			<button
				type="button"
				onClick={() => setIsExpanded(!isExpanded)}
				className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
			>
				{isExpanded ? (
					<ChevronDown className="h-3 w-3" />
				) : (
					<ChevronRight className="h-3 w-3" />
				)}
				<span className="font-medium">{label}</span>
				{itemCount !== undefined && (
					<span className="text-muted-foreground/70">({itemCount} items)</span>
				)}
			</button>
			{isExpanded && (
				<pre className="mt-2 text-xs font-mono text-muted-foreground bg-muted/30 rounded p-2 overflow-x-auto">
					{JSON.stringify(data, null, 2)}
				</pre>
			)}
		</div>
	);
}

// Extract text content from a value - handles nested objects like message.content
function extractTextContent(value: unknown): string | null {
	if (typeof value === "string") {
		return value;
	}
	if (isJsonObject(value)) {
		// Look for content field in nested objects (e.g., message.content)
		if (typeof value.content === "string") {
			return value.content;
		}
		// Also check for text field
		if (typeof value.text === "string") {
			return value.text;
		}
	}
	return null;
}

function JsonMessageCard({ data }: { data: JsonObject }) {
	// Extract badges
	const badges: Array<{ key: string; value: string }> = [];
	for (const field of BADGE_FIELDS) {
		if (data[field] && typeof data[field] === "string") {
			badges.push({ key: field, value: data[field] as string });
		}
	}

	// Extract main text - check for nested content in objects
	let mainText: string | null = null;
	let mainTextField: string | null = null;
	for (const field of TEXT_FIELDS) {
		const extracted = extractTextContent(data[field]);
		if (extracted) {
			mainText = extracted;
			mainTextField = field;
			break;
		}
	}

	// Extract key-value pairs
	const keyValues: Array<{ key: string; value: string }> = [];
	const collapsibles: Array<{
		key: string;
		data: unknown;
		itemCount?: number;
	}> = [];

	for (const [key, value] of Object.entries(data)) {
		if (SKIP_FIELDS.includes(key)) continue;
		if (BADGE_FIELDS.includes(key)) continue;
		// Skip the field we extracted main text from
		if (key === mainTextField) continue;

		// Handle collapsible fields (both arrays and objects)
		if (COLLAPSIBLE_FIELDS.includes(key)) {
			if (Array.isArray(value) && value.length > 0) {
				collapsibles.push({ key, data: value, itemCount: value.length });
			} else if (isJsonObject(value) && Object.keys(value).length > 0) {
				collapsibles.push({ key, data: value });
			}
			continue;
		}

		// Format the value
		if (typeof value === "string") {
			// Skip empty strings
			if (value === "") continue;
			keyValues.push({ key, value });
		} else if (typeof value === "number" || typeof value === "boolean") {
			keyValues.push({ key, value: String(value) });
		} else if (value === null) {
			// Skip null values
			continue;
		} else if (Array.isArray(value) && value.length === 0) {
			// Skip empty arrays
			continue;
		} else if (isJsonObject(value) && Object.keys(value).length === 0) {
			// Skip empty objects
			continue;
		} else {
			// For other complex values, stringify them
			keyValues.push({ key, value: JSON.stringify(value) });
		}
	}

	return (
		<div className="bg-muted/20 rounded-lg p-3 space-y-2">
			{/* Badges row */}
			{badges.length > 0 && (
				<div className="flex flex-wrap gap-1.5">
					{badges.map(({ key, value }) => (
						<Badge key={key} variant="secondary" className="text-xs">
							{value}
						</Badge>
					))}
				</div>
			)}

			{/* Main text */}
			{mainText && (
				<div className="text-sm text-foreground whitespace-pre-wrap">
					{mainText}
				</div>
			)}

			{/* Key-value table */}
			{keyValues.length > 0 && (
				<div className="grid gap-1 text-xs">
					{keyValues.map(({ key, value }) => (
						<div key={key} className="flex gap-2">
							<span className="text-muted-foreground min-w-[120px] shrink-0">
								{key}:
							</span>
							<span className="font-mono text-foreground/80 break-all">
								{value}
							</span>
						</div>
					))}
				</div>
			)}

			{/* Collapsible sections */}
			{collapsibles.map(({ key, data, itemCount }) => (
				<CollapsibleSection
					key={key}
					label={key}
					data={data}
					itemCount={itemCount}
				/>
			))}
		</div>
	);
}

// Try to parse JSON that may span multiple lines
function parseJsonLines(
	content: string,
): Array<{ type: "json"; data: JsonObject } | { type: "text"; text: string }> {
	const messages: Array<
		{ type: "json"; data: JsonObject } | { type: "text"; text: string }
	> = [];
	const lines = content.split("\n");
	let jsonBuffer = "";
	let braceCount = 0;

	for (const line of lines) {
		const trimmed = line.trim();
		if (!trimmed) continue;

		// If we're accumulating JSON, continue until balanced braces
		if (jsonBuffer || trimmed.startsWith("{")) {
			jsonBuffer += (jsonBuffer ? "\n" : "") + line;

			// Count braces (simple approach, doesn't handle strings perfectly)
			for (const char of trimmed) {
				if (char === "{") braceCount++;
				else if (char === "}") braceCount--;
			}

			if (braceCount === 0 && jsonBuffer) {
				// Try to parse the accumulated JSON
				try {
					const parsed = JSON.parse(jsonBuffer) as JsonObject;
					messages.push({ type: "json", data: parsed });
				} catch {
					// Failed to parse, treat as text
					messages.push({ type: "text", text: jsonBuffer });
				}
				jsonBuffer = "";
			}
		} else {
			messages.push({ type: "text", text: line });
		}
	}

	// Handle any remaining buffer
	if (jsonBuffer) {
		try {
			const parsed = JSON.parse(jsonBuffer) as JsonObject;
			messages.push({ type: "json", data: parsed });
		} catch {
			messages.push({ type: "text", text: jsonBuffer });
		}
	}

	return messages;
}

export function JsonMessageRenderer({ content }: JsonMessageRendererProps) {
	if (!content) return null;

	const messages = parseJsonLines(content);

	if (messages.length === 0) return null;

	// Check if we have any JSON messages
	const hasJson = messages.some((m) => m.type === "json");

	// If no JSON, just show as plain text
	if (!hasJson) {
		return (
			<pre className="font-mono text-sm whitespace-pre-wrap break-words text-foreground/80">
				{content}
			</pre>
		);
	}

	return (
		<div className="space-y-2">
			{messages.map((msg, index) => {
				if (msg.type === "json") {
					return <JsonMessageCard key={index} data={msg.data} />;
				}
				return (
					<pre
						key={index}
						className="font-mono text-sm whitespace-pre-wrap break-words text-foreground/80"
					>
						{msg.text}
					</pre>
				);
			})}
		</div>
	);
}
