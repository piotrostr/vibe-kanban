import { useState, useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import {
	ChevronDown,
	ChevronRight,
	Terminal,
	CheckCircle,
	XCircle,
	User,
	Settings,
	AlertTriangle,
} from "lucide-react";
import { cn } from "@/lib/utils";

type JsonObject = Record<string, unknown>;

interface JsonMessageRendererProps {
	content: string;
}

function isJsonObject(value: unknown): value is JsonObject {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Extract text content from message objects
function extractMessageText(msg: JsonObject): string | null {
	// Direct content field
	if (typeof msg.content === "string") {
		return msg.content;
	}
	// Nested message.content
	if (isJsonObject(msg.message)) {
		const inner = msg.message as JsonObject;
		if (Array.isArray(inner.content)) {
			// Handle content array: [{"type":"text","text":"..."}]
			const texts = inner.content
				.filter(
					(c): c is { type: string; text: string } =>
						isJsonObject(c) && c.type === "text" && typeof c.text === "string",
				)
				.map((c) => c.text);
			if (texts.length > 0) return texts.join("");
		}
		if (typeof inner.content === "string") {
			return inner.content;
		}
		if (typeof inner.text === "string") {
			return inner.text;
		}
	}
	// Direct text field
	if (typeof msg.text === "string") {
		return msg.text;
	}
	return null;
}

// Extract text from streaming delta events
function extractTextFromStreamEvent(data: JsonObject): string | null {
	// Handle {"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"..."}}}
	if (data.type === "stream_event" && isJsonObject(data.event)) {
		const event = data.event as JsonObject;
		if (event.type === "content_block_delta" && isJsonObject(event.delta)) {
			const delta = event.delta as JsonObject;
			if (delta.type === "text_delta" && typeof delta.text === "string") {
				return delta.text;
			}
		}
	}
	// Direct content_block_delta
	if (data.type === "content_block_delta" && isJsonObject(data.delta)) {
		const delta = data.delta as JsonObject;
		if (delta.type === "text_delta" && typeof delta.text === "string") {
			return delta.text;
		}
	}
	return null;
}

// Check if this is a streaming protocol event we should skip
function isStreamingProtocolEvent(data: JsonObject): boolean {
	const eventType = data.type as string;

	// Direct streaming event types
	if (
		eventType === "stream_event" ||
		eventType === "content_block_start" ||
		eventType === "content_block_delta" ||
		eventType === "content_block_stop" ||
		eventType === "message_start" ||
		eventType === "message_delta" ||
		eventType === "message_stop" ||
		eventType === "ping"
	) {
		return true;
	}

	// Check for nested stream_event
	if (data.stream_event !== undefined) {
		return true;
	}

	// Check for event field containing streaming types
	if (isJsonObject(data.event)) {
		const event = data.event as JsonObject;
		const innerType = event.type as string;
		if (
			innerType === "content_block_start" ||
			innerType === "content_block_delta" ||
			innerType === "content_block_stop" ||
			innerType === "message_start" ||
			innerType === "message_delta" ||
			innerType === "message_stop" ||
			innerType === "input_json_delta"
		) {
			return true;
		}
	}

	return false;
}

// Parsed message types for Claude Code-style rendering
type ParsedMessage =
	| { type: "system_init"; data: JsonObject }
	| { type: "user_message"; text: string; isSynthetic?: boolean }
	| { type: "assistant_message"; text: string }
	| { type: "tool_use"; name: string; input?: JsonObject }
	| { type: "tool_result"; content: string; isError?: boolean }
	| { type: "result"; status: "success" | "error"; data: JsonObject }
	| { type: "aggregated_text"; text: string }
	| { type: "stop_hook"; text: string }
	| { type: "json"; data: JsonObject };

// Parse JSON lines into Claude Code-style messages
function parseJsonLines(content: string): ParsedMessage[] {
	const rawMessages: Array<
		{ type: "json"; data: JsonObject } | { type: "text"; text: string }
	> = [];
	const lines = content.split("\n");
	let jsonBuffer = "";
	let braceCount = 0;

	for (const line of lines) {
		const trimmed = line.trim();
		if (!trimmed) continue;

		if (jsonBuffer || trimmed.startsWith("{")) {
			jsonBuffer += (jsonBuffer ? "\n" : "") + line;

			for (const char of trimmed) {
				if (char === "{") braceCount++;
				else if (char === "}") braceCount--;
			}

			if (braceCount === 0 && jsonBuffer) {
				try {
					const parsed = JSON.parse(jsonBuffer) as JsonObject;
					rawMessages.push({ type: "json", data: parsed });
				} catch {
					// Failed to parse - skip if it looks like broken protocol JSON
					const isProtocolNoise =
						jsonBuffer.includes('"type":"stream_event"') ||
						jsonBuffer.includes('"type":"stream_') ||
						jsonBuffer.includes('"type":"assistant"') ||
						jsonBuffer.includes('"type":"user"') ||
						jsonBuffer.includes('"type":"system"');
					if (!isProtocolNoise) {
						rawMessages.push({ type: "text", text: jsonBuffer });
					}
				}
				jsonBuffer = "";
			}
		} else {
			rawMessages.push({ type: "text", text: line });
		}
	}

	if (jsonBuffer) {
		try {
			const parsed = JSON.parse(jsonBuffer) as JsonObject;
			rawMessages.push({ type: "json", data: parsed });
		} catch {
			// Skip broken protocol JSON
			const isProtocolNoise =
				jsonBuffer.includes('"type":"stream_event"') ||
				jsonBuffer.includes('"type":"stream_') ||
				jsonBuffer.includes('"type":"assistant"') ||
				jsonBuffer.includes('"type":"user"') ||
				jsonBuffer.includes('"type":"system"');
			if (!isProtocolNoise) {
				rawMessages.push({ type: "text", text: jsonBuffer });
			}
		}
	}

	// Process into Claude Code-style messages
	const result: ParsedMessage[] = [];
	let aggregatedText = "";

	// Check if raw text looks like broken protocol JSON
	const isBrokenProtocolJson = (text: string): boolean => {
		const t = text.trim();
		// Broken JSON fragments from protocol messages
		if (t.includes('"type":"stream_event"')) return true;
		if (t.includes('"type":"stream_')) return true;
		if (t.includes('"type":"assistant"')) return true;
		if (t.includes('"type":"user"')) return true;
		if (t.includes('"type":"system"')) return true;
		if (t.startsWith('event","event"')) return true;
		if (t.startsWith('n_id":"')) return true; // broken session_id
		if (t.startsWith('":{"input_tokens"')) return true; // broken usage
		if (t.startsWith('text":"')) return true; // broken text field
		return false;
	};

	for (const msg of rawMessages) {
		if (msg.type === "text") {
			// Skip broken protocol JSON fragments
			if (isBrokenProtocolJson(msg.text)) {
				continue;
			}
			// Plain text lines
			if (aggregatedText) {
				result.push({ type: "aggregated_text", text: aggregatedText });
				aggregatedText = "";
			}
			result.push({ type: "aggregated_text", text: msg.text });
			continue;
		}

		const data = msg.data;

		// Extract streaming text
		const streamText = extractTextFromStreamEvent(data);
		if (streamText !== null) {
			aggregatedText += streamText;
			continue;
		}

		// Skip other streaming protocol events
		if (isStreamingProtocolEvent(data)) {
			continue;
		}

		// Flush aggregated text
		if (aggregatedText) {
			result.push({ type: "aggregated_text", text: aggregatedText });
			aggregatedText = "";
		}

		// Handle system init: {"type":"system","subtype":"init",...}
		if (data.type === "system" && data.subtype === "init") {
			result.push({ type: "system_init", data });
			continue;
		}

		// Handle user message: {"type":"user","message":{...}}
		if (data.type === "user") {
			const text = extractMessageText(data);
			if (text) {
				// Check for stop hook feedback
				if (text.startsWith("Stop hook feedback:")) {
					result.push({
						type: "stop_hook",
						text: text.replace("Stop hook feedback:\n\n", ""),
					});
				} else {
					result.push({
						type: "user_message",
						text,
						isSynthetic: data.isSynthetic === true,
					});
				}
				continue;
			}
		}

		// Handle assistant message: {"type":"assistant","message":{...}}
		if (data.type === "assistant") {
			const text = extractMessageText(data);
			if (text) {
				result.push({ type: "assistant_message", text });
				continue;
			}
			// Check for tool_use in assistant message content
			if (isJsonObject(data.message)) {
				const msg = data.message as JsonObject;
				if (Array.isArray(msg.content)) {
					for (const item of msg.content) {
						if (isJsonObject(item) && item.type === "tool_use") {
							result.push({
								type: "tool_use",
								name: item.name as string,
								input: item.input as JsonObject | undefined,
							});
						}
					}
					continue;
				}
			}
		}

		// Handle tool_result in user messages
		if (data.type === "user" && isJsonObject(data.message)) {
			const msg = data.message as JsonObject;
			if (Array.isArray(msg.content)) {
				for (const item of msg.content) {
					if (isJsonObject(item) && item.type === "tool_result") {
						const content =
							typeof item.content === "string"
								? item.content
								: JSON.stringify(item.content);
						result.push({
							type: "tool_result",
							content,
							isError: item.is_error === true,
						});
					}
				}
				continue;
			}
		}

		// Handle result: {"type":"result","result":"success",...} or {"result":"success",...}
		if (data.result === "success" || data.result === "error") {
			result.push({
				type: "result",
				status: data.result as "success" | "error",
				data,
			});
			continue;
		}

		// Default: show as JSON card
		result.push({ type: "json", data });
	}

	// Flush remaining aggregated text
	if (aggregatedText) {
		result.push({ type: "aggregated_text", text: aggregatedText });
	}

	// Simple deduplication: skip consecutive messages with identical text content
	const deduplicated: ParsedMessage[] = [];
	const getTextContent = (msg: ParsedMessage): string | null => {
		switch (msg.type) {
			case "aggregated_text":
				return msg.text.trim();
			case "assistant_message":
				return msg.text.trim();
			case "stop_hook":
				return msg.text.trim();
			case "user_message":
				return msg.text.trim();
			default:
				return null;
		}
	};

	for (const msg of result) {
		const lastMsg = deduplicated[deduplicated.length - 1];
		if (lastMsg) {
			const currentText = getTextContent(msg);
			const lastText = getTextContent(lastMsg);
			// Skip if both have text and it's identical
			if (currentText && lastText && currentText === lastText) {
				continue;
			}
		}
		deduplicated.push(msg);
	}

	return deduplicated;
}

// Collapsible section for arrays/objects
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
		<div className="mt-1">
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
				<pre className="mt-1 text-xs font-mono text-muted-foreground bg-muted/30 rounded p-2 overflow-x-auto max-h-[200px]">
					{JSON.stringify(data, null, 2)}
				</pre>
			)}
		</div>
	);
}

// System init card - like Claude Code's init block
function SystemInitCard({ data }: { data: JsonObject }) {
	const [expanded, setExpanded] = useState(false);

	// Key fields to show inline
	const inlineFields = [
		"cwd",
		"session_id",
		"model",
		"permissionMode",
		"claude_code_version",
	];
	// Fields to collapse
	const collapsibleFields = [
		"tools",
		"mcp_servers",
		"slash_commands",
		"agents",
		"skills",
		"plugins",
		"usage",
	];

	const inlineEntries = inlineFields
		.filter((k) => data[k] !== undefined && data[k] !== null)
		.map((k) => ({ key: k, value: String(data[k]) }));

	const collapsibleEntries = collapsibleFields
		.filter((k) => data[k] !== undefined && data[k] !== null)
		.map((k) => ({
			key: k,
			data: data[k],
			count: Array.isArray(data[k]) ? (data[k] as unknown[]).length : undefined,
		}));

	return (
		<div className="border border-border/50 rounded-md overflow-hidden bg-muted/20">
			<button
				type="button"
				onClick={() => setExpanded(!expanded)}
				className="w-full flex items-center gap-2 px-3 py-2 text-sm bg-muted/30 hover:bg-muted/50 transition-colors"
			>
				<Settings className="h-3.5 w-3.5 text-muted-foreground" />
				<span className="font-medium">system</span>
				<Badge variant="secondary" className="text-[10px] px-1.5 py-0">
					init
				</Badge>
				<span className="ml-auto">
					{expanded ? (
						<ChevronDown className="h-3 w-3" />
					) : (
						<ChevronRight className="h-3 w-3" />
					)}
				</span>
			</button>
			{expanded && (
				<div className="px-3 py-2 text-xs space-y-1 border-t border-border/30">
					{inlineEntries.map(({ key, value }) => (
						<div key={key} className="flex gap-2">
							<span className="text-muted-foreground min-w-[100px]">
								{key}:
							</span>
							<span className="font-mono text-foreground/80 break-all">
								{value}
							</span>
						</div>
					))}
					{collapsibleEntries.map(({ key, data, count }) => (
						<CollapsibleSection
							key={key}
							label={key}
							data={data}
							itemCount={count}
						/>
					))}
				</div>
			)}
		</div>
	);
}

// Result card
function ResultCard({
	status,
	data,
}: {
	status: "success" | "error";
	data: JsonObject;
}) {
	const [expanded, setExpanded] = useState(false);

	// Extract useful fields
	const duration = data.duration_ms as number | undefined;
	const numTurns = data.num_turns as number | undefined;
	const cost = data.total_cost_usd as number | undefined;

	return (
		<div
			className={cn(
				"border rounded-md overflow-hidden",
				status === "success"
					? "border-green-500/30 bg-green-500/5"
					: "border-red-500/30 bg-red-500/5",
			)}
		>
			<button
				type="button"
				onClick={() => setExpanded(!expanded)}
				className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-muted/20 transition-colors"
			>
				{status === "success" ? (
					<CheckCircle className="h-3.5 w-3.5 text-green-500" />
				) : (
					<XCircle className="h-3.5 w-3.5 text-red-500" />
				)}
				<span className="font-medium">result</span>
				<Badge
					variant="secondary"
					className={cn(
						"text-[10px] px-1.5 py-0",
						status === "success"
							? "bg-green-500/20 text-green-700"
							: "bg-red-500/20 text-red-700",
					)}
				>
					{status}
				</Badge>
				{duration !== undefined && (
					<span className="text-muted-foreground text-xs ml-2">
						{duration}ms
					</span>
				)}
				{numTurns !== undefined && (
					<span className="text-muted-foreground text-xs">
						{numTurns} turns
					</span>
				)}
				{cost !== undefined && cost > 0 && (
					<span className="text-muted-foreground text-xs">
						${cost.toFixed(4)}
					</span>
				)}
				<span className="ml-auto">
					{expanded ? (
						<ChevronDown className="h-3 w-3" />
					) : (
						<ChevronRight className="h-3 w-3" />
					)}
				</span>
			</button>
			{expanded && (
				<div className="px-3 py-2 text-xs font-mono border-t border-border/30 max-h-[150px] overflow-y-auto">
					<pre className="whitespace-pre-wrap break-all">
						{JSON.stringify(data, null, 2)}
					</pre>
				</div>
			)}
		</div>
	);
}

// User message card
function UserMessageCard({
	text,
	isSynthetic,
}: {
	text: string;
	isSynthetic?: boolean;
}) {
	return (
		<div className="flex gap-2 items-start">
			<User className="h-4 w-4 text-muted-foreground mt-0.5 flex-shrink-0" />
			<div className="text-sm">
				{isSynthetic && (
					<span className="text-xs text-muted-foreground mr-2">
						(synthetic)
					</span>
				)}
				<span className="whitespace-pre-wrap">{text}</span>
			</div>
		</div>
	);
}

// Assistant message card - bullet style like Claude Code
function AssistantMessageCard({ text }: { text: string }) {
	return (
		<div className="flex gap-2 items-start">
			<span className="text-blue-500 mt-0.5 flex-shrink-0">⏺</span>
			<div className="text-sm whitespace-pre-wrap">{text}</div>
		</div>
	);
}

// Stop hook card
function StopHookCard({ text }: { text: string }) {
	return (
		<div className="flex gap-2 items-start pl-4 border-l-2 border-amber-500/50">
			<AlertTriangle className="h-3.5 w-3.5 text-amber-500 mt-0.5 flex-shrink-0" />
			<div className="text-xs text-muted-foreground">
				<span className="font-medium">Stop says:</span> {text}
			</div>
		</div>
	);
}

// Get a short summary for tool input - Claude Code style
function getToolSummary(name: string, input?: JsonObject): string {
	if (!input) return "";

	switch (name) {
		case "Bash":
			// Show the command, truncated
			if (typeof input.command === "string") {
				const cmd = input.command;
				return cmd.length > 80 ? cmd.slice(0, 80) + "..." : cmd;
			}
			break;
		case "Read":
			// Show file path
			if (typeof input.file_path === "string") {
				return input.file_path;
			}
			break;
		case "Write":
		case "Edit":
			// Show file path
			if (typeof input.file_path === "string") {
				return input.file_path;
			}
			break;
		case "Glob":
			// Show pattern
			if (typeof input.pattern === "string") {
				return input.pattern;
			}
			break;
		case "Grep":
			// Show pattern
			if (typeof input.pattern === "string") {
				return input.pattern;
			}
			break;
	}
	return "";
}

// Tool use card - Claude Code style: ⏺ ToolName(summary)
function ToolUseCard({ name, input }: { name: string; input?: JsonObject }) {
	const [expanded, setExpanded] = useState(false);
	const summary = getToolSummary(name, input);

	return (
		<div className="flex gap-2 items-start">
			<span className="text-blue-500 mt-0.5 flex-shrink-0">⏺</span>
			<div className="flex-1 min-w-0">
				<button
					type="button"
					onClick={() => input && setExpanded(!expanded)}
					className="text-sm font-mono text-muted-foreground hover:text-foreground text-left"
				>
					<span className="font-medium text-foreground">{name}</span>
					{summary && (
						<span className="text-muted-foreground">({summary})</span>
					)}
					{input && !summary && (
						<span className="ml-1">
							{expanded ? (
								<ChevronDown className="h-3 w-3 inline" />
							) : (
								<ChevronRight className="h-3 w-3 inline" />
							)}
						</span>
					)}
				</button>
				{expanded && input && (
					<pre className="mt-1 text-xs font-mono text-muted-foreground bg-muted/30 rounded p-2 overflow-x-auto max-h-[150px]">
						{JSON.stringify(input, null, 2)}
					</pre>
				)}
			</div>
		</div>
	);
}

// Tool result card
function ToolResultCard({
	content,
	isError,
}: {
	content: string;
	isError?: boolean;
}) {
	const [expanded, setExpanded] = useState(false);
	const isLong = content.length > 200;
	const preview = isLong ? content.slice(0, 200) + "..." : content;

	return (
		<div
			className={cn(
				"pl-6 text-xs font-mono",
				isError ? "text-red-500" : "text-muted-foreground",
			)}
		>
			{isLong ? (
				<button
					type="button"
					onClick={() => setExpanded(!expanded)}
					className="text-left hover:bg-muted/20 rounded p-1 -m-1"
				>
					<pre className="whitespace-pre-wrap break-all">
						{expanded ? content : preview}
					</pre>
				</button>
			) : (
				<pre className="whitespace-pre-wrap break-all">{content}</pre>
			)}
		</div>
	);
}

// Generic JSON card for unrecognized messages
function JsonCard({ data }: { data: JsonObject }) {
	const [expanded, setExpanded] = useState(false);

	// Extract type/subtype for badge
	const msgType = data.type as string | undefined;
	const subtype = data.subtype as string | undefined;

	// Try to extract meaningful text
	const text = extractMessageText(data);

	return (
		<div className="border border-border/50 rounded-md overflow-hidden bg-muted/10">
			<button
				type="button"
				onClick={() => setExpanded(!expanded)}
				className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-muted/20 transition-colors"
			>
				<Terminal className="h-3.5 w-3.5 text-muted-foreground" />
				{msgType && (
					<Badge variant="secondary" className="text-[10px] px-1.5 py-0">
						{msgType}
					</Badge>
				)}
				{subtype && (
					<Badge variant="outline" className="text-[10px] px-1.5 py-0">
						{subtype}
					</Badge>
				)}
				{text && (
					<span className="text-muted-foreground text-xs truncate max-w-[200px]">
						{text.slice(0, 50)}...
					</span>
				)}
				<span className="ml-auto">
					{expanded ? (
						<ChevronDown className="h-3 w-3" />
					) : (
						<ChevronRight className="h-3 w-3" />
					)}
				</span>
			</button>
			{expanded && (
				<div className="px-3 py-2 text-xs font-mono border-t border-border/30 max-h-[200px] overflow-y-auto">
					<pre className="whitespace-pre-wrap break-all">
						{JSON.stringify(data, null, 2)}
					</pre>
				</div>
			)}
		</div>
	);
}

export function JsonMessageRenderer({ content }: JsonMessageRendererProps) {
	const messages = useMemo(() => {
		if (!content) return [];
		return parseJsonLines(content);
	}, [content]);

	if (messages.length === 0) return null;

	return (
		<div className="space-y-2">
			{messages.map((msg, index) => {
				switch (msg.type) {
					case "system_init":
						return <SystemInitCard key={index} data={msg.data} />;
					case "user_message":
						return (
							<UserMessageCard
								key={index}
								text={msg.text}
								isSynthetic={msg.isSynthetic}
							/>
						);
					case "assistant_message":
						return <AssistantMessageCard key={index} text={msg.text} />;
					case "tool_use":
						return (
							<ToolUseCard key={index} name={msg.name} input={msg.input} />
						);
					case "tool_result":
						return (
							<ToolResultCard
								key={index}
								content={msg.content}
								isError={msg.isError}
							/>
						);
					case "result":
						return (
							<ResultCard key={index} status={msg.status} data={msg.data} />
						);
					case "stop_hook":
						return <StopHookCard key={index} text={msg.text} />;
					case "aggregated_text":
						return (
							<div key={index} className="text-sm whitespace-pre-wrap">
								{msg.text}
							</div>
						);
					case "json":
						return <JsonCard key={index} data={msg.data} />;
					default:
						return null;
				}
			})}
		</div>
	);
}
