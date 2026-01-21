#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::external::LinearIssue;
use crate::state::{Task, TaskStatus};

/// File-based task storage.
/// Tasks are stored as markdown files in ~/.vibe/projects/{project}/tasks/
#[derive(Debug)]
pub struct TaskStorage {
    tasks_dir: PathBuf,
    project_name: String,
}

/// Frontmatter parsed from task markdown files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskFrontmatter {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_labels: Option<String>,
    pub created: String,
}

impl TaskStorage {
    /// Create storage for the current working directory's project
    pub fn from_cwd() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let project_name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))?
            .to_string();

        Self::new(&project_name)
    }

    /// Create storage for a specific project name
    pub fn new(project_name: &str) -> Result<Self> {
        let vibe_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("No home directory"))?
            .join(".vibe");

        let tasks_dir = vibe_dir
            .join("projects")
            .join(project_name)
            .join("tasks");

        std::fs::create_dir_all(&tasks_dir)
            .with_context(|| format!("Failed to create tasks directory: {:?}", tasks_dir))?;

        Ok(Self {
            tasks_dir,
            project_name: project_name.to_string(),
        })
    }

    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    pub fn tasks_dir(&self) -> &PathBuf {
        &self.tasks_dir
    }

    /// List all tasks from markdown files
    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let pattern = format!("{}/*.md", self.tasks_dir.display());
        let paths: Vec<PathBuf> = glob::glob(&pattern)
            .context("Failed to read glob pattern")?
            .filter_map(Result::ok)
            .collect();

        let mut tasks = Vec::with_capacity(paths.len());
        for path in paths {
            match self.parse_task(&path) {
                Ok(task) => tasks.push(task),
                Err(e) => {
                    tracing::warn!("Failed to parse task file {:?}: {}", path, e);
                }
            }
        }

        // Sort by created date (newest first)
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(tasks)
    }

    /// Create a new task
    pub fn create_task(&self, title: &str, description: Option<&str>) -> Result<Task> {
        let id = uuid::Uuid::new_v4().to_string();
        let slug = slugify(title);
        let path = self.tasks_dir.join(format!("{}.md", slug));

        // Handle duplicate filenames
        let path = if path.exists() {
            let short_id = &id[..8];
            self.tasks_dir.join(format!("{}-{}.md", slug, short_id))
        } else {
            path
        };

        let created = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let frontmatter = TaskFrontmatter {
            id: id.clone(),
            linear_id: None,
            linear_url: None,
            linear_labels: None,
            created: created.clone(),
        };

        let content = format!(
            "---\n{}---\n\n# {}\n\n{}",
            serde_yaml::to_string(&frontmatter).unwrap_or_default(),
            title,
            description.unwrap_or("")
        );

        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write task file: {:?}", path))?;

        Ok(Task {
            id,
            project_id: self.project_name.clone(),
            title: title.to_string(),
            description: description.map(String::from),
            status: TaskStatus::Backlog,
            parent_workspace_id: None,
            shared_task_id: None,
            linear_issue_id: None,
            linear_url: None,
            linear_labels: None,
            created_at: created.clone(),
            updated_at: created,
            has_in_progress_attempt: false,
            last_attempt_failed: false,
            executor: String::new(),
            pr_url: None,
            pr_status: None,
            pr_is_draft: None,
            pr_review_decision: None,
            pr_checks_status: None,
            pr_has_conflicts: None,
        })
    }

    /// Create a task from a Linear issue
    pub fn create_task_from_linear(&self, issue: &LinearIssue) -> Result<Task> {
        let id = uuid::Uuid::new_v4().to_string();
        let slug = slugify(&issue.title);
        let path = self.tasks_dir.join(format!("{}.md", slug));

        // Handle duplicate filenames
        let path = if path.exists() {
            let short_id = &id[..8];
            self.tasks_dir.join(format!("{}-{}.md", slug, short_id))
        } else {
            path
        };

        let created = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let labels_str = if issue.labels.is_empty() {
            None
        } else {
            Some(issue.labels.join(", "))
        };

        let frontmatter = TaskFrontmatter {
            id: id.clone(),
            linear_id: Some(issue.id.clone()),
            linear_url: Some(issue.url.clone()),
            linear_labels: labels_str.clone(),
            created: created.clone(),
        };

        let content = format!(
            "---\n{}---\n\n# {}\n\n{}",
            serde_yaml::to_string(&frontmatter).unwrap_or_default(),
            issue.title,
            issue.description.as_deref().unwrap_or("")
        );

        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write task file: {:?}", path))?;

        Ok(Task {
            id,
            project_id: self.project_name.clone(),
            title: issue.title.clone(),
            description: issue.description.clone(),
            status: TaskStatus::Backlog,
            parent_workspace_id: None,
            shared_task_id: None,
            linear_issue_id: Some(issue.id.clone()),
            linear_url: Some(issue.url.clone()),
            linear_labels: labels_str,
            created_at: created.clone(),
            updated_at: created,
            has_in_progress_attempt: false,
            last_attempt_failed: false,
            executor: String::new(),
            pr_url: None,
            pr_status: None,
            pr_is_draft: None,
            pr_review_decision: None,
            pr_checks_status: None,
            pr_has_conflicts: None,
        })
    }

    /// Update an existing task
    pub fn update_task(&self, task_id: &str, title: &str, description: Option<&str>) -> Result<Task> {
        let (path, mut frontmatter) = self.find_task_file(task_id)?;

        let content = format!(
            "---\n{}---\n\n# {}\n\n{}",
            serde_yaml::to_string(&frontmatter).unwrap_or_default(),
            title,
            description.unwrap_or("")
        );

        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write task file: {:?}", path))?;

        // Rename file if title changed significantly
        let new_slug = slugify(title);
        let new_filename = format!("{}.md", new_slug);
        let current_filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        if !current_filename.starts_with(&new_slug) {
            let new_path = self.tasks_dir.join(&new_filename);
            if !new_path.exists() {
                std::fs::rename(&path, &new_path)?;
            }
        }

        Ok(Task {
            id: task_id.to_string(),
            project_id: self.project_name.clone(),
            title: title.to_string(),
            description: description.map(String::from),
            status: TaskStatus::Backlog,
            parent_workspace_id: None,
            shared_task_id: None,
            linear_issue_id: frontmatter.linear_id.take(),
            linear_url: frontmatter.linear_url.take(),
            linear_labels: frontmatter.linear_labels.take(),
            created_at: frontmatter.created.clone(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            has_in_progress_attempt: false,
            last_attempt_failed: false,
            executor: String::new(),
            pr_url: None,
            pr_status: None,
            pr_is_draft: None,
            pr_review_decision: None,
            pr_checks_status: None,
            pr_has_conflicts: None,
        })
    }

    /// Delete a task by ID
    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let (path, _) = self.find_task_file(task_id)?;
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to delete task file: {:?}", path))?;
        Ok(())
    }

    /// Find task file by ID
    fn find_task_file(&self, task_id: &str) -> Result<(PathBuf, TaskFrontmatter)> {
        let pattern = format!("{}/*.md", self.tasks_dir.display());
        for entry in glob::glob(&pattern).context("Failed to read glob pattern")? {
            let path = entry?;
            if let Ok((frontmatter, _, _)) = self.parse_task_content(&path) {
                if frontmatter.id == task_id {
                    return Ok((path, frontmatter));
                }
            }
        }
        anyhow::bail!("Task not found: {}", task_id)
    }

    /// Parse a task from a markdown file
    fn parse_task(&self, path: &PathBuf) -> Result<Task> {
        let (frontmatter, title, description) = self.parse_task_content(path)?;

        Ok(Task {
            id: frontmatter.id,
            project_id: self.project_name.clone(),
            title,
            description,
            status: TaskStatus::Backlog, // Status derived from git/PR state
            parent_workspace_id: None,
            shared_task_id: None,
            linear_issue_id: frontmatter.linear_id,
            linear_url: frontmatter.linear_url,
            linear_labels: frontmatter.linear_labels,
            created_at: frontmatter.created.clone(),
            updated_at: frontmatter.created,
            has_in_progress_attempt: false,
            last_attempt_failed: false,
            executor: String::new(),
            pr_url: None,
            pr_status: None,
            pr_is_draft: None,
            pr_review_decision: None,
            pr_checks_status: None,
            pr_has_conflicts: None,
        })
    }

    /// Parse task content from a file
    fn parse_task_content(&self, path: &PathBuf) -> Result<(TaskFrontmatter, String, Option<String>)> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read task file: {:?}", path))?;

        // Parse frontmatter
        let (frontmatter, body) = if content.starts_with("---") {
            let parts: Vec<&str> = content.splitn(3, "---").collect();
            if parts.len() >= 3 {
                let yaml = parts[1].trim();
                let body = parts[2].trim();
                let fm: TaskFrontmatter = serde_yaml::from_str(yaml)
                    .unwrap_or_else(|_| TaskFrontmatter {
                        id: uuid::Uuid::new_v4().to_string(),
                        linear_id: None,
                        linear_url: None,
                        linear_labels: None,
                        created: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    });
                (fm, body.to_string())
            } else {
                (TaskFrontmatter::default(), content)
            }
        } else {
            (TaskFrontmatter::default(), content)
        };

        // Parse title from first heading
        let mut lines = body.lines();
        let title = lines
            .find(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        // Rest is description
        let description: String = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        let description = if description.is_empty() {
            None
        } else {
            Some(description)
        };

        Ok((frontmatter, title, description))
    }
}

/// Convert a title to a filename-safe slug
fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Add feature: user auth"), "add-feature-user-auth");
        assert_eq!(slugify("Fix bug #123"), "fix-bug-123");
        assert_eq!(slugify("  Spaces  everywhere  "), "spaces-everywhere");
    }

    #[test]
    fn test_parse_frontmatter() {
        let yaml = r#"
id: abc123
linear_id: TEAM-456
created: 2024-01-15
"#;
        let fm: TaskFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(fm.id, "abc123");
        assert_eq!(fm.linear_id, Some("TEAM-456".to_string()));
        assert_eq!(fm.created, "2024-01-15");
    }
}
