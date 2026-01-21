use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::ClaudeActivityState;

const STALE_THRESHOLD_SECS: u64 = 2;

#[derive(Debug, Deserialize)]
struct ClaudeStatusFile {
    working_dir: String,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct TokenSnapshot {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

pub struct ClaudeActivityTracker {
    state_dir: PathBuf,
    previous_snapshots: HashMap<String, TokenSnapshot>,
}

impl ClaudeActivityTracker {
    pub fn new() -> Self {
        let state_dir = dirs::home_dir()
            .map(|h| h.join(".vibe").join("claude-activity"))
            .unwrap_or_else(|| PathBuf::from("/tmp/claude-activity"));

        Self {
            state_dir,
            previous_snapshots: HashMap::new(),
        }
    }

    pub fn get_activity_for_session(&mut self, session_name: &str) -> ClaudeActivityState {
        // Try to find a status file that matches this session name
        // The status file is named by MD5 hash of the working directory
        // We need to scan all files and match by session name in the working_dir

        let Ok(entries) = fs::read_dir(&self.state_dir) else {
            return ClaudeActivityState::Unknown;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(status) = serde_json::from_str::<ClaudeStatusFile>(&content) {
                        // Check if this status file's working_dir contains the session name
                        // Session names are typically derived from branch names or directory names
                        if self.session_matches_working_dir(session_name, &status.working_dir) {
                            return self.determine_state(&status);
                        }
                    }
                }
            }
        }

        ClaudeActivityState::Unknown
    }

    fn session_matches_working_dir(&self, session_name: &str, working_dir: &str) -> bool {
        // Session names are sanitized versions of branch names
        // Check if the working directory path contains the session name
        let normalized_session = session_name.to_lowercase();
        let normalized_dir = working_dir.to_lowercase();

        // Check if the directory ends with the session name (worktree scenario)
        if let Some(last_component) = working_dir.split('/').last() {
            if last_component.to_lowercase() == normalized_session {
                return true;
            }
        }

        // Check if the session name is contained in the directory path
        normalized_dir.contains(&normalized_session)
    }

    fn determine_state(&mut self, status: &ClaudeStatusFile) -> ClaudeActivityState {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Check if status is stale (>2s old)
        if now.saturating_sub(status.timestamp) > STALE_THRESHOLD_SECS {
            return ClaudeActivityState::Idle;
        }

        // If tokens are null, we have no session data
        let (Some(input), Some(output)) = (status.input_tokens, status.output_tokens) else {
            return ClaudeActivityState::Unknown;
        };

        let current_snapshot = TokenSnapshot {
            input_tokens: Some(input),
            output_tokens: Some(output),
        };

        // Check against previous snapshot
        let state = if let Some(prev) = self.previous_snapshots.get(&status.working_dir) {
            // Compare tokens - if changed, Claude is thinking
            if prev.input_tokens != current_snapshot.input_tokens
                || prev.output_tokens != current_snapshot.output_tokens
            {
                ClaudeActivityState::Thinking
            } else {
                // Tokens unchanged - Claude is waiting for user
                ClaudeActivityState::WaitingForUser
            }
        } else {
            // First time seeing this session - assume thinking if we have tokens
            ClaudeActivityState::Thinking
        };

        // Update snapshot
        self.previous_snapshots
            .insert(status.working_dir.clone(), current_snapshot);

        state
    }

    pub fn update_sessions(&mut self, sessions: &mut [super::ZellijSession]) {
        for session in sessions.iter_mut() {
            session.claude_activity = self.get_activity_for_session(&session.name);
        }
    }
}

impl Default for ClaudeActivityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
fn hash_working_dir(working_dir: &str) -> String {
    format!("{:x}", md5::compute(working_dir.as_bytes()))
        .chars()
        .take(16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_working_dir() {
        let hash = hash_working_dir("/Users/test/project");
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_session_matches_working_dir() {
        let tracker = ClaudeActivityTracker::new();

        // Exact match at end of path
        assert!(tracker.session_matches_working_dir(
            "feature-branch",
            "/Users/test/worktrees/feature-branch"
        ));

        // Session name contained in path
        assert!(tracker.session_matches_working_dir(
            "my-feature",
            "/Users/test/my-feature-worktree"
        ));

        // Case insensitive
        assert!(tracker.session_matches_working_dir(
            "Feature-Branch",
            "/users/test/feature-branch"
        ));

        // No match
        assert!(!tracker.session_matches_working_dir(
            "other-branch",
            "/Users/test/feature-branch"
        ));
    }
}
