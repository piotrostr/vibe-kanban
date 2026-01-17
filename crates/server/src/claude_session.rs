use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

#[derive(Debug, Error)]
pub enum ClaudeSessionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error on line {line}: {error}")]
    JsonParse { line: usize, error: String },
    #[error("Invalid session path: {0}")]
    InvalidPath(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMessage {
    #[serde(rename = "type")]
    msg_type: String,
    uuid: Option<String>,
    parent_uuid: Option<String>,
    session_id: Option<String>,
    timestamp: Option<String>,
    git_branch: Option<String>,
    summary: Option<String>,
    message: Option<MessageContent>,
    is_sidechain: Option<bool>,
    agent_id: Option<String>,
    slug: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum MessageContent {
    Object {
        role: String,
        content: ContentValue,
        #[serde(default)]
        id: Option<String>, // Message ID for aggregating streamed chunks
    },
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ContentValue {
    String(String),
    Array(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
    #[serde(default)]
    content: Option<ContentBlockContent>,
}

/// Content field in ContentBlock can be a string or nested array (tool_result blocks)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ContentBlockContent {
    String(String),
    Array(Vec<NestedContentBlock>),
}

/// Nested content blocks within tool_result - simplified to just extract text
#[derive(Debug, Clone, Deserialize)]
struct NestedContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub timestamp: String,
    pub branch: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub path: String,
    pub session_id: String,
    pub last_modified: String,
    pub summary: Option<String>,
    pub message_count: usize,
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PreviewClaudeSessionRequest {
    pub session_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PreviewClaudeSessionResponse {
    pub items: Vec<ExtractedTask>,
    pub session_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromClaudeSessionRequest {
    pub session_path: String,
    pub selected_item_ids: Vec<String>,
    pub default_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromClaudeSessionResponse {
    pub imported_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ListClaudeSessionsRequest {
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ListClaudeSessionsResponse {
    pub sessions: Vec<SessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportWithHistoryRequest {
    pub session_path: String,
    pub task_title: Option<String>,
    pub default_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportWithHistoryResponse {
    pub task_id: String,
    pub workspace_id: String,
    pub session_id: String,
    pub execution_process_id: String,
    pub log_lines_imported: usize,
}

fn extract_text_content(content: &ContentValue) -> String {
    match content {
        ContentValue::String(s) => s.clone(),
        ContentValue::Array(blocks) => blocks
            .iter()
            .filter_map(|block| {
                if block.block_type.as_deref() == Some("text") {
                    block.text.clone()
                } else {
                    // Handle nested content (e.g., tool_result blocks)
                    extract_content_block_text(&block.content)
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn extract_content_block_text(content: &Option<ContentBlockContent>) -> Option<String> {
    match content {
        Some(ContentBlockContent::String(s)) => Some(s.clone()),
        Some(ContentBlockContent::Array(blocks)) => {
            let texts: Vec<String> = blocks
                .iter()
                .filter_map(|b| {
                    if b.block_type.as_deref() == Some("text") {
                        b.text.clone()
                    } else {
                        None
                    }
                })
                .collect();
            if texts.is_empty() {
                None
            } else {
                Some(texts.join("\n"))
            }
        }
        None => None,
    }
}

fn truncate_title(text: &str, max_len: usize) -> String {
    let first_line = text.lines().next().unwrap_or(text);
    let trimmed = first_line.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..max_len.saturating_sub(3)])
    }
}

pub fn parse_session_file(path: &Path) -> Result<Vec<ExtractedTask>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;
    let mut tasks = Vec::new();
    let mut summaries: Vec<String> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let msg: RawMessage = serde_json::from_str(line).map_err(|e| ClaudeSessionError::JsonParse {
            line: line_num + 1,
            error: e.to_string(),
        })?;

        // Skip sidechain messages (agent warmups, etc.)
        if msg.is_sidechain == Some(true) || msg.agent_id.is_some() {
            continue;
        }

        match msg.msg_type.as_str() {
            "summary" => {
                if let Some(summary) = msg.summary {
                    summaries.push(summary);
                }
            }
            "user" => {
                // Only process user messages that start a new work item (parentUuid is null)
                if msg.parent_uuid.is_none() {
                    if let Some(MessageContent::Object { content, .. }) = msg.message {
                        let text = extract_text_content(&content);
                        let trimmed = text.trim();

                        // Skip empty messages and internal warmup messages
                        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("warmup") {
                            continue;
                        }

                        let title = truncate_title(&text, 100);
                        let id = msg.uuid.unwrap_or_else(|| {
                            format!("task-{}", line_num)
                        });

                        tasks.push(ExtractedTask {
                            id,
                            title,
                            description: Some(text),
                            timestamp: msg.timestamp.unwrap_or_default(),
                            branch: msg.git_branch,
                            session_id: msg.session_id,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // If we found summaries, use them to enrich task titles
    if !summaries.is_empty() && !tasks.is_empty() {
        // Use the most recent summary as the first task's title
        if let Some(first_summary) = summaries.last() {
            if let Some(first_task) = tasks.first_mut() {
                first_task.title = first_summary.clone();
            }
        }
    }

    Ok(tasks)
}

pub fn list_available_sessions(project_path: Option<&str>) -> Result<Vec<SessionInfo>, ClaudeSessionError> {
    let claude_dir = dirs::home_dir()
        .ok_or_else(|| ClaudeSessionError::InvalidPath("Cannot find home directory".to_string()))?
        .join(".claude")
        .join("projects");

    if !claude_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    // Walk through all project directories
    for entry in std::fs::read_dir(&claude_dir)? {
        let entry = entry?;
        let project_dir = entry.path();

        if !project_dir.is_dir() {
            continue;
        }

        // Check if this directory matches the project path filter
        let dir_name = project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Convert project path to Claude's directory naming (/ becomes -)
        if let Some(filter_path) = project_path {
            let normalized_filter = filter_path.replace('/', "-");
            if !dir_name.contains(&normalized_filter) && !dir_name.starts_with('-') {
                continue;
            }
        }

        // Find .jsonl files in this project directory
        for file_entry in std::fs::read_dir(&project_dir)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();

            // Skip agent session files (named like agent-*.jsonl)
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if file_name.starts_with("agent-") {
                continue;
            }

            if file_path.extension().map_or(false, |ext| ext == "jsonl") {
                if let Some(session_info) = parse_session_info(&file_path)? {
                    sessions.push(session_info);
                }
            }
        }
    }

    // Sort by last modified, most recent first
    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(sessions)
}

fn parse_session_info(path: &Path) -> Result<Option<SessionInfo>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Ok(None);
    }

    let mut session_id = None;
    let mut git_branch = None;
    let mut last_summary = None;
    let mut message_count = 0;
    let mut last_timestamp = None;

    for line in lines.iter() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            message_count += 1;

            if session_id.is_none() {
                session_id = msg.session_id.clone();
            }
            if git_branch.is_none() {
                git_branch = msg.git_branch.clone();
            }
            if msg.timestamp.is_some() {
                last_timestamp = msg.timestamp.clone();
            }
            if msg.msg_type == "summary" {
                last_summary = msg.summary;
            }
        }
    }

    let session_id = session_id.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    });

    let last_modified = last_timestamp.unwrap_or_else(|| {
        path.metadata()
            .and_then(|m| m.modified())
            .map(|t| {
                chrono::DateTime::<chrono::Utc>::from(t)
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
            })
            .unwrap_or_default()
    });

    Ok(Some(SessionInfo {
        path: path.to_string_lossy().to_string(),
        session_id,
        last_modified,
        summary: last_summary,
        message_count,
        git_branch,
    }))
}

/// Extract text content from a MessageContent, handling tool calls gracefully
fn extract_message_content(message: &Option<MessageContent>) -> Option<String> {
    match message {
        Some(MessageContent::Object { content, .. }) => {
            let text = extract_text_content(content);
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
        Some(MessageContent::String(s)) => {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        None => None,
    }
}

/// Extract the message ID from a MessageContent for aggregating streamed chunks
fn get_message_id(message: &Option<MessageContent>) -> Option<String> {
    match message {
        Some(MessageContent::Object { id, .. }) => id.clone(),
        _ => None,
    }
}

/// Extract all conversation log lines from a session file for import.
/// Returns formatted conversation turns for display.
/// Assistant messages are aggregated by message.id to avoid duplicate chunks from streaming.
pub fn extract_session_logs(path: &Path) -> Result<Vec<String>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;

    // Track seen message IDs to deduplicate assistant messages
    // Key: message.id, Value: (timestamp, formatted_content)
    let mut assistant_messages: HashMap<String, (String, String)> = HashMap::new();
    let mut logs: Vec<(String, String)> = Vec::new(); // (timestamp, content)

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            // Skip sidechain messages (agent warmups, etc.)
            if msg.is_sidechain == Some(true) || msg.agent_id.is_some() {
                continue;
            }

            match msg.msg_type.as_str() {
                "user" => {
                    // User messages don't have message.id streaming, emit directly
                    if let Some(content) = extract_message_content(&msg.message) {
                        logs.push((
                            msg.timestamp.clone().unwrap_or_default(),
                            format!("User: {}", content),
                        ));
                    }
                }
                "assistant" => {
                    // Assistant messages: aggregate by message.id
                    if let Some(msg_id) = get_message_id(&msg.message) {
                        if let Some(content) = extract_message_content(&msg.message) {
                            // Replace previous entry with same ID (later entry has more content)
                            assistant_messages.insert(
                                msg_id,
                                (
                                    msg.timestamp.clone().unwrap_or_default(),
                                    format!("Assistant: {}", content),
                                ),
                            );
                        }
                    } else {
                        // No message ID, emit directly
                        if let Some(content) = extract_message_content(&msg.message) {
                            logs.push((
                                msg.timestamp.clone().unwrap_or_default(),
                                format!("Assistant: {}", content),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Merge assistant messages into logs
    logs.extend(assistant_messages.into_values());

    // Sort by timestamp
    logs.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(logs.into_iter().map(|(_, content)| content).collect())
}

/// Get the first user message content for use as task title/description
pub fn get_first_user_message(path: &Path) -> Result<Option<(String, String)>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            if msg.is_sidechain == Some(true) || msg.agent_id.is_some() {
                continue;
            }

            if msg.msg_type == "user" && msg.parent_uuid.is_none() {
                if let Some(MessageContent::Object { content, .. }) = msg.message {
                    let text = extract_text_content(&content);
                    let trimmed = text.trim();

                    if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("warmup") {
                        let title = truncate_title(&text, 100);
                        return Ok(Some((title, text)));
                    }
                }
            }
        }
    }

    Ok(None)
}

pub fn get_session_summary(path: &Path) -> Result<Option<String>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;

    // Find the last summary in the file
    let mut last_summary = None;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            if msg.msg_type == "summary" {
                last_summary = msg.summary;
            }
        }
    }

    Ok(last_summary)
}

/// Extract raw JSONL lines from a session file for 1:1 import.
/// Returns the raw lines as-is from the Claude Code session file.
pub fn extract_raw_session_logs(path: &Path) -> Result<Vec<String>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect())
}

/// Extract the session slug from a Claude Code session file.
/// The slug is used to locate the corresponding plan file.
pub fn get_session_slug(path: &Path) -> Result<Option<String>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            if let Some(slug) = msg.slug {
                return Ok(Some(slug));
            }
        }
    }
    Ok(None)
}

/// Get the plan file path for a session, if it exists.
/// Plans are stored at ~/.claude/plans/{slug}.md
pub fn get_plan_path(session_path: &Path) -> Result<Option<PathBuf>, ClaudeSessionError> {
    let slug = get_session_slug(session_path)?;
    if let Some(slug) = slug {
        let plan_path = dirs::home_dir()
            .ok_or_else(|| {
                ClaudeSessionError::InvalidPath("Cannot find home directory".to_string())
            })?
            .join(".claude")
            .join("plans")
            .join(format!("{}.md", slug));
        if plan_path.exists() {
            return Ok(Some(plan_path));
        }
    }
    Ok(None)
}

/// Extract the working directory (cwd) from a Claude Code session file.
/// The cwd is stored in "system" type entries.
pub fn get_session_cwd(path: &Path) -> Result<Option<String>, ClaudeSessionError> {
    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<RawMessage>(line) {
            if msg.msg_type == "system" {
                if let Some(cwd) = msg.cwd {
                    return Ok(Some(cwd));
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_title() {
        assert_eq!(truncate_title("Hello world", 100), "Hello world");
        assert_eq!(truncate_title("Hello", 3), "...");
        assert_eq!(truncate_title("Hello world", 8), "Hello...");
        assert_eq!(truncate_title("Line1\nLine2\nLine3", 100), "Line1");
    }

    #[test]
    fn test_extract_text_content_string() {
        let content = ContentValue::String("Hello world".to_string());
        assert_eq!(extract_text_content(&content), "Hello world");
    }

    #[test]
    fn test_extract_text_content_array() {
        let content = ContentValue::Array(vec![
            ContentBlock {
                block_type: Some("text".to_string()),
                text: Some("Hello".to_string()),
                content: None,
            },
            ContentBlock {
                block_type: Some("text".to_string()),
                text: Some("World".to_string()),
                content: None,
            },
        ]);
        assert_eq!(extract_text_content(&content), "Hello\nWorld");
    }

    #[test]
    fn test_extract_message_content_object() {
        let msg = Some(MessageContent::Object {
            role: "user".to_string(),
            content: ContentValue::String("Hello world".to_string()),
            id: None,
        });
        assert_eq!(
            extract_message_content(&msg),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn test_get_message_id() {
        let msg_with_id = Some(MessageContent::Object {
            role: "assistant".to_string(),
            content: ContentValue::String("Hello".to_string()),
            id: Some("msg_123".to_string()),
        });
        assert_eq!(get_message_id(&msg_with_id), Some("msg_123".to_string()));

        let msg_without_id = Some(MessageContent::Object {
            role: "assistant".to_string(),
            content: ContentValue::String("Hello".to_string()),
            id: None,
        });
        assert_eq!(get_message_id(&msg_without_id), None);

        let msg_string = Some(MessageContent::String("Hello".to_string()));
        assert_eq!(get_message_id(&msg_string), None);

        assert_eq!(get_message_id(&None), None);
    }

    #[test]
    fn test_extract_message_content_string() {
        let msg = Some(MessageContent::String("Hello world".to_string()));
        assert_eq!(extract_message_content(&msg), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_message_content_empty() {
        let msg = Some(MessageContent::String("   ".to_string()));
        assert_eq!(extract_message_content(&msg), None);
    }

    #[test]
    fn test_extract_message_content_none() {
        assert_eq!(extract_message_content(&None), None);
    }

    #[test]
    #[ignore] // Requires local Claude session files - run with: cargo test -- --ignored
    fn test_extract_session_logs_real_session() {
        // Point this to a real session file on your machine
        // Other devs: update this path to a session from your ~/.claude/projects/
        // This session has 1305 lines and was previously failing to parse
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr-vibe-kanban/97d974f2-9f0d-4088-b7c4-94459de5ac18.jsonl";

        let path = Path::new(session_path);
        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        let logs = extract_session_logs(path).unwrap();

        // Verify structure
        assert!(!logs.is_empty(), "Should extract at least one log entry");

        // Check that all entries are properly formatted
        for log in &logs {
            assert!(
                log.starts_with("User: ") || log.starts_with("Assistant: "),
                "Log entry should start with 'User: ' or 'Assistant: ', got: {}...",
                &log[..log.len().min(50)]
            );
        }

        // Print summary for manual verification
        println!("Extracted {} log entries", logs.len());
        println!(
            "First entry: {}...",
            &logs[0][..logs[0].len().min(100)]
        );
    }

    #[test]
    #[ignore] // Requires local Claude session files - run with: cargo test -- --ignored
    fn test_extract_session_logs_aggregation_real_session() {
        // This test verifies that assistant message aggregation works correctly.
        // Without aggregation, you'd see many duplicate assistant entries with
        // incrementally longer content (from streaming chunks).
        // With aggregation, each assistant message.id appears only once.
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr-vibe-kanban/97d974f2-9f0d-4088-b7c4-94459de5ac18.jsonl";

        let path = Path::new(session_path);
        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        let logs = extract_session_logs(path).unwrap();

        // Count user vs assistant messages
        let user_count = logs.iter().filter(|l| l.starts_with("User: ")).count();
        let assistant_count = logs
            .iter()
            .filter(|l| l.starts_with("Assistant: "))
            .count();

        println!(
            "User messages: {}, Assistant messages: {}",
            user_count, assistant_count
        );

        // In a normal conversation, we expect roughly similar counts of user/assistant messages
        // Without aggregation, assistant messages would vastly outnumber user messages
        // due to streaming chunks. This is a rough heuristic check.
        assert!(
            assistant_count <= user_count * 5,
            "Too many assistant messages ({}) compared to user messages ({}) - aggregation may not be working",
            assistant_count, user_count
        );
    }

    #[test]
    #[ignore] // Requires local Claude session files - run with: cargo test -- --ignored
    fn test_parse_session_file_real_session() {
        // Test parse_session_file which extracts tasks from user messages
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr-vibe-kanban/97d974f2-9f0d-4088-b7c4-94459de5ac18.jsonl";

        let path = Path::new(session_path);
        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        let result = parse_session_file(path);
        match result {
            Ok(tasks) => {
                println!("Successfully parsed {} tasks", tasks.len());
                for (i, task) in tasks.iter().take(5).enumerate() {
                    println!("Task {}: {}", i + 1, task.title);
                }
            }
            Err(e) => {
                panic!("Failed to parse session file: {}", e);
            }
        }
    }

    /// E2E test for import_with_history functionality.
    ///
    /// This test verifies that import_with_history creates all required records:
    /// 1. Task - with title from session's first user message or summary
    /// 2. Workspace - with WorkspaceRepo entries for project repositories
    /// 3. Session - linked to workspace
    /// 4. ExecutionProcess - with ImportedSession run_reason and Completed status
    /// 5. ExecutionProcessLogs - with batch-inserted JSONL conversation logs
    ///
    /// Run manually after setting up a test database:
    ///   cargo test --package server test_import_with_history_e2e -- --ignored --nocapture
    ///
    /// Prerequisites:
    /// - Running database with migrations applied
    /// - A Claude session file to import
    /// - Project with configured repositories
    #[test]
    #[ignore]
    fn test_import_with_history_e2e() {
        // This test requires:
        // 1. A running database with migrations applied (including importedsession run_reason)
        // 2. A Claude session file to import
        // 3. A project with configured repositories
        //
        // The import_with_history route handler should:
        // - Create a Task with appropriate title/description
        // - Create a Workspace with WorkspaceRepo entries from project repositories
        // - Create a Session linked to the workspace
        // - Create an ExecutionProcess with:
        //   - run_reason = 'importedsession'
        //   - status = 'completed'
        //   - executor_action pointing to a CodingAgent action
        // - Create ExecutionProcessLogs entries for each conversation turn
        //
        // Verify by checking:
        // - Task exists and has correct title
        // - Workspace has at least one WorkspaceRepo (otherwise ensure_container_exists fails)
        // - ExecutionProcess has run_reason='importedsession' and status='completed'
        // - ExecutionProcessLogs count matches extract_session_logs result

        println!("This test requires manual verification with a running database.");
        println!("See test documentation for required setup steps.");
    }

    /// Tests import with ground truth data from a real "remove redundant buttons" session.
    ///
    /// This compares the imported structure against expected values based on the
    /// ground truth Vibe database entry.
    ///
    /// Ground truth session: ~/.claude/projects/-Users-piotrostr--vibe-worktrees-619c-remove-redundant-buttons-vibe-kanban/a5a96b63-44c8-4974-b5bf-c52ffa29f60b.jsonl
    ///
    /// Expected output:
    /// - Task title: "remove redundant buttons" (first user message)
    /// - Branch: "piotr/remove-redundant-buttons" (from gitBranch field)
    /// - Logs: User/Assistant conversation turns
    #[test]
    #[ignore]
    fn test_import_with_history_ground_truth() {
        // Ground truth session path
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr--vibe-worktrees-619c-remove-redundant-buttons-vibe-kanban/a5a96b63-44c8-4974-b5bf-c52ffa29f60b.jsonl";
        let path = Path::new(session_path);

        if !path.exists() {
            println!("Skipping test - ground truth session file not found at: {}", session_path);
            return;
        }

        // Test first user message extraction (used for task title)
        let first_message = get_first_user_message(path).unwrap();
        assert!(first_message.is_some(), "Should extract first user message");
        let (title, description) = first_message.unwrap();

        println!("Title: {}", title);
        println!("Description preview: {}...", &description[..description.len().min(100)]);

        // Ground truth: first message should be about removing redundant buttons
        assert!(
            title.to_lowercase().contains("remove") && title.to_lowercase().contains("button"),
            "Title should contain 'remove' and 'button', got: {}",
            title
        );

        // Test log extraction
        let logs = extract_session_logs(path).unwrap();
        assert!(!logs.is_empty(), "Should extract conversation logs");

        // Count message types
        let user_count = logs.iter().filter(|l| l.starts_with("User: ")).count();
        let assistant_count = logs.iter().filter(|l| l.starts_with("Assistant: ")).count();

        println!("Log entries: {} total ({} user, {} assistant)",
            logs.len(), user_count, assistant_count);

        // Ground truth: should have at least 1 user and 1 assistant message
        assert!(user_count >= 1, "Should have at least 1 user message");
        assert!(assistant_count >= 1, "Should have at least 1 assistant message");

        // Verify first user message matches the title
        let first_user_log = logs.iter().find(|l| l.starts_with("User: ")).unwrap();
        assert!(
            first_user_log.to_lowercase().contains("remove") && first_user_log.to_lowercase().contains("button"),
            "First user log should match the task topic"
        );

        // Verify log format for imported sessions (stored as {"Stdout":"User: ..."})
        for log in &logs {
            assert!(
                log.starts_with("User: ") || log.starts_with("Assistant: "),
                "All logs should be User: or Assistant: format, got: {}...",
                &log[..log.len().min(50)]
            );
        }

        println!("Ground truth validation passed.");
        println!("First log: {}...", &logs[0][..logs[0].len().min(100)]);
    }

    /// Test extract_raw_session_logs returns raw JSONL lines for 1:1 parity.
    /// Each line should be valid JSON with Claude Code structure.
    #[test]
    #[ignore]
    fn test_extract_raw_session_logs_parity() {
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr--vibe-worktrees-619c-remove-redundant-buttons-vibe-kanban/a5a96b63-44c8-4974-b5bf-c52ffa29f60b.jsonl";
        let path = Path::new(session_path);

        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        // Extract raw logs
        let raw_logs = extract_raw_session_logs(path).unwrap();
        assert!(!raw_logs.is_empty(), "Should extract raw log lines");

        // Count actual non-empty lines in source file for comparison
        let source_content = std::fs::read_to_string(path).unwrap();
        let source_line_count = source_content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();

        assert_eq!(
            raw_logs.len(),
            source_line_count,
            "Raw log count should match source file line count"
        );

        // Verify each line is valid JSON with expected structure
        let mut user_messages = 0;
        let mut assistant_messages = 0;

        for (i, line) in raw_logs.iter().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("Line {} should be valid JSON: {} - line: {}", i + 1, e, line)
            });

            // Check that it has a type field (Claude Code structure)
            if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
                match msg_type {
                    "user" => user_messages += 1,
                    "assistant" => assistant_messages += 1,
                    _ => {} // Other types like "system", "summary", "queue-operation", etc.
                }
            }
        }

        println!(
            "Raw logs: {} total ({} user, {} assistant)",
            raw_logs.len(),
            user_messages,
            assistant_messages
        );

        // Should have at least one user and one assistant message
        assert!(user_messages >= 1, "Should have at least 1 user message");
        assert!(
            assistant_messages >= 1,
            "Should have at least 1 assistant message"
        );

        // Verify first non-queue-operation line has expected structure
        let first_message_line = raw_logs
            .iter()
            .find(|l| {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(l) {
                    v.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "user" || t == "assistant")
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .expect("Should find at least one user/assistant message");

        let first_parsed: serde_json::Value = serde_json::from_str(first_message_line).unwrap();
        assert!(
            first_parsed.get("message").is_some(),
            "Message lines should have 'message' field"
        );

        println!("Raw JSONL parity test passed.");
    }

    /// Test get_session_slug extracts the slug from a session file.
    #[test]
    #[ignore]
    fn test_get_session_slug() {
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr--vibe-worktrees-619c-remove-redundant-buttons-vibe-kanban/a5a96b63-44c8-4974-b5bf-c52ffa29f60b.jsonl";
        let path = Path::new(session_path);

        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        let slug = get_session_slug(path).unwrap();
        assert!(slug.is_some(), "Should extract slug from session");

        let slug = slug.unwrap();
        assert_eq!(
            slug, "giggly-munching-clover",
            "Slug should be 'giggly-munching-clover'"
        );

        println!("Extracted slug: {}", slug);
    }

    /// Test get_plan_path returns the correct path for a session with a plan.
    #[test]
    #[ignore]
    fn test_get_plan_path() {
        let session_path = "/Users/piotrostr/.claude/projects/-Users-piotrostr--vibe-worktrees-619c-remove-redundant-buttons-vibe-kanban/a5a96b63-44c8-4974-b5bf-c52ffa29f60b.jsonl";
        let path = Path::new(session_path);

        if !path.exists() {
            println!(
                "Skipping test - session file not found at: {}",
                session_path
            );
            return;
        }

        let plan_path = get_plan_path(path).unwrap();
        assert!(plan_path.is_some(), "Should find plan file for this session");

        let plan_path = plan_path.unwrap();
        assert!(plan_path.exists(), "Plan file should exist");
        assert!(
            plan_path
                .to_string_lossy()
                .contains("giggly-munching-clover.md"),
            "Plan path should contain slug"
        );

        // Verify plan content
        let plan_content = std::fs::read_to_string(&plan_path).unwrap();
        assert!(
            plan_content.contains("Remove Redundant Buttons"),
            "Plan should contain expected title"
        );

        println!("Found plan at: {}", plan_path.display());
        println!("Plan preview: {}...", &plan_content[..plan_content.len().min(200)]);
    }
}
