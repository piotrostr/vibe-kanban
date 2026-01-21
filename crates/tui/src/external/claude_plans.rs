use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use serde::Deserialize;

/// Reads Claude Code plans from session files.
///
/// Claude Code stores plans in `~/.claude/plans/` and references them via `planFilePath`
/// in session JSONL files at `~/.claude/projects/{sanitized-path}/`.
pub struct ClaudePlanReader {
    projects_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SessionEntry {
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    #[serde(rename = "planFilePath")]
    plan_file_path: Option<String>,
}

impl ClaudePlanReader {
    pub fn new() -> Self {
        let projects_dir = dirs::home_dir()
            .map(|h| h.join(".claude").join("projects"))
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        Self { projects_dir }
    }

    /// Find the plan for a specific branch in a project.
    pub fn find_plan_for_branch(&self, project_path: &str, branch: &str) -> Option<String> {
        let sanitized = sanitize_project_path(project_path);
        let project_dir = self.projects_dir.join(&sanitized);

        if !project_dir.exists() {
            return None;
        }

        let Ok(entries) = fs::read_dir(&project_dir) else {
            return None;
        };

        // Collect session files with their modification times for sorting
        let mut session_files: Vec<_> = entries
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((e.path(), modified))
            })
            .collect();

        // Sort by modification time, newest first
        session_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Check sessions from newest to oldest
        for (path, _) in session_files {
            if let Some((session_branch, plan_path)) = self.extract_plan_from_session(&path) {
                if session_branch == branch {
                    return self.read_plan_file(&plan_path);
                }
            }
        }

        None
    }

    /// Extract branch and plan path from a session JSONL file.
    /// Returns the last entry that has both a branch and plan path.
    fn extract_plan_from_session(&self, path: &PathBuf) -> Option<(String, String)> {
        let file = fs::File::open(path).ok()?;
        let reader = BufReader::new(file);

        let mut result: Option<(String, String)> = None;

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<SessionEntry>(&line) {
                if let (Some(branch), Some(plan_path)) = (entry.git_branch, entry.plan_file_path) {
                    if !branch.is_empty() && !plan_path.is_empty() {
                        result = Some((branch, plan_path));
                    }
                }
            }
        }

        result
    }

    /// Read the content of a plan file.
    fn read_plan_file(&self, path: &str) -> Option<String> {
        let plan_path = PathBuf::from(path);
        if plan_path.exists() {
            fs::read_to_string(&plan_path).ok()
        } else {
            None
        }
    }
}

impl Default for ClaudePlanReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize a project path to match Claude Code's directory naming.
/// Claude replaces path separators with dashes.
fn sanitize_project_path(path: &str) -> String {
    path.replace('/', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_project_path() {
        assert_eq!(
            sanitize_project_path("/Users/test/my-project"),
            "-Users-test-my-project"
        );
        assert_eq!(
            sanitize_project_path("/home/user/code/app"),
            "-home-user-code-app"
        );
    }

    #[test]
    fn test_reader_creation() {
        let reader = ClaudePlanReader::new();
        assert!(reader.projects_dir.to_string_lossy().contains(".claude"));
    }
}
