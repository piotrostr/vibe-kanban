use std::path::Path;

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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum MessageContent {
    Object { role: String, content: ContentValue },
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
    content: Option<String>,
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

fn extract_text_content(content: &ContentValue) -> String {
    match content {
        ContentValue::String(s) => s.clone(),
        ContentValue::Array(blocks) => blocks
            .iter()
            .filter_map(|block| {
                if block.block_type.as_deref() == Some("text") {
                    block.text.clone()
                } else {
                    block.content.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
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
}
