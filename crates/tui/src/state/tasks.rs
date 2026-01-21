use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Backlog,
    Todo,
    Inprogress,
    Inreview,
    Done,
    Cancelled,
}

impl TaskStatus {
    // All statuses for serialization/backend
    pub const ALL: [TaskStatus; 6] = [
        TaskStatus::Backlog,
        TaskStatus::Todo,
        TaskStatus::Inprogress,
        TaskStatus::Inreview,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ];

    // Visible columns in the TUI (skip Todo and Cancelled to save space)
    pub const VISIBLE: [TaskStatus; 4] = [
        TaskStatus::Backlog,
        TaskStatus::Inprogress,
        TaskStatus::Inreview,
        TaskStatus::Done,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TaskStatus::Backlog => "Backlog",
            TaskStatus::Todo => "To Do",
            TaskStatus::Inprogress => "In Progress",
            TaskStatus::Inreview => "In Review",
            TaskStatus::Done => "Done",
            TaskStatus::Cancelled => "Cancelled",
        }
    }

    // Column index for visible columns only
    pub fn column_index(&self) -> usize {
        match self {
            TaskStatus::Backlog => 0,
            TaskStatus::Todo => 0, // Map to Backlog column
            TaskStatus::Inprogress => 1,
            TaskStatus::Inreview => 2,
            TaskStatus::Done => 3,
            TaskStatus::Cancelled => 3, // Map to Done column
        }
    }

    pub fn from_column_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(TaskStatus::Backlog),
            1 => Some(TaskStatus::Inprogress),
            2 => Some(TaskStatus::Inreview),
            3 => Some(TaskStatus::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub parent_workspace_id: Option<String>,
    pub shared_task_id: Option<String>,
    pub linear_issue_id: Option<String>,
    pub linear_url: Option<String>,
    pub linear_labels: Option<String>,
    pub created_at: String,
    pub updated_at: String,

    // Attempt status fields
    #[serde(default)]
    pub has_in_progress_attempt: bool,
    #[serde(default)]
    pub last_attempt_failed: bool,
    #[serde(default)]
    pub executor: String,
    pub pr_url: Option<String>,
    pub pr_status: Option<String>,
    pub pr_is_draft: Option<bool>,
    pub pr_review_decision: Option<String>,
    pub pr_checks_status: Option<String>,
    pub pr_has_conflicts: Option<bool>,
}

use crate::external::BranchPrInfo;

impl Task {
    /// Compute the effective display status based on PR status.
    /// - PR merged -> Done
    /// - PR closed -> Cancelled (maps to Done column)
    /// - PR open (not draft) -> InReview
    /// - Otherwise, use the task's stored status
    pub fn effective_status(&self) -> TaskStatus {
        if let Some(ref pr_status) = self.pr_status {
            match pr_status.as_str() {
                "merged" => return TaskStatus::Done,
                "closed" => return TaskStatus::Cancelled,
                "open" => {
                    // Draft PRs stay in their current status
                    if self.pr_is_draft != Some(true) {
                        return TaskStatus::Inreview;
                    }
                }
                _ => {}
            }
        }
        self.status
    }

    /// Compute effective status considering locally detected PR info and worktree
    /// Priority: PR status > worktree exists > stored status
    pub fn effective_status_with_pr(
        &self,
        branch_pr: Option<&BranchPrInfo>,
        has_worktree: bool,
    ) -> TaskStatus {
        // Backend PR info takes precedence
        if self.pr_status.is_some() {
            return self.effective_status();
        }

        // Check locally detected PR
        if let Some(pr) = branch_pr {
            match pr.state.as_str() {
                "MERGED" => return TaskStatus::Done,
                "CLOSED" => return TaskStatus::Cancelled,
                "OPEN" => {
                    if !pr.is_draft {
                        return TaskStatus::Inreview;
                    }
                    // Draft PR with worktree -> In Progress
                    if has_worktree {
                        return TaskStatus::Inprogress;
                    }
                }
                _ => {}
            }
        }

        // Worktree exists means we're working on it -> In Progress
        if has_worktree {
            return TaskStatus::Inprogress;
        }

        self.status
    }
}

const NUM_VISIBLE_COLUMNS: usize = 4;

pub struct TasksState {
    pub tasks: Vec<Task>,
    pub selected_column: usize,
    pub selected_card_per_column: [usize; NUM_VISIBLE_COLUMNS],
    pub loading: bool,
    pub search_filter: String,
}

impl TasksState {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_column: 0, // Start on Backlog
            selected_card_per_column: [0; NUM_VISIBLE_COLUMNS],
            loading: false,
            search_filter: String::new(),
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        // Reset card selections
        self.selected_card_per_column = [0; NUM_VISIBLE_COLUMNS];
    }

    pub fn tasks_in_column(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks_in_column_with_prs(status, &std::collections::HashMap::new(), &[])
    }

    /// Get tasks in a column, considering locally detected PR status for transitions
    pub fn tasks_in_column_with_prs(
        &self,
        status: TaskStatus,
        branch_prs: &std::collections::HashMap<String, BranchPrInfo>,
        worktrees: &[crate::external::WorktreeInfo],
    ) -> Vec<&Task> {
        let column_index = status.column_index();
        self.tasks
            .iter()
            .filter(|t| {
                // Find matching worktree for this task
                let task_slug = t.title.to_lowercase().replace(' ', "-");
                let matching_branch = worktrees.iter().find(|w| {
                    w.branch.to_lowercase().contains(&task_slug)
                        || task_slug.contains(&w.branch.to_lowercase())
                });

                let has_worktree = matching_branch.is_some();
                let branch_pr = matching_branch.and_then(|wt| branch_prs.get(&wt.branch));
                t.effective_status_with_pr(branch_pr, has_worktree).column_index() == column_index
            })
            .filter(|t| {
                if self.search_filter.is_empty() {
                    return true;
                }
                let query = self.search_filter.to_lowercase();
                t.title.to_lowercase().contains(&query)
                    || t.description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.selected_task_with_prs(&std::collections::HashMap::new(), &[])
    }

    pub fn selected_task_with_prs(
        &self,
        branch_prs: &std::collections::HashMap<String, BranchPrInfo>,
        worktrees: &[crate::external::WorktreeInfo],
    ) -> Option<&Task> {
        let status = TaskStatus::from_column_index(self.selected_column)?;
        let tasks = self.tasks_in_column_with_prs(status, branch_prs, worktrees);
        let card_index = self.selected_card_per_column[self.selected_column];
        tasks.get(card_index).copied()
    }

    pub fn select_next_card(&mut self) {
        self.select_next_card_with_prs(&std::collections::HashMap::new(), &[]);
    }

    pub fn select_next_card_with_prs(
        &mut self,
        branch_prs: &std::collections::HashMap<String, BranchPrInfo>,
        worktrees: &[crate::external::WorktreeInfo],
    ) {
        if let Some(status) = TaskStatus::from_column_index(self.selected_column) {
            let count = self.tasks_in_column_with_prs(status, branch_prs, worktrees).len();
            if count > 0 {
                let current = self.selected_card_per_column[self.selected_column];
                self.selected_card_per_column[self.selected_column] = (current + 1) % count;
            }
        }
    }

    pub fn select_prev_card(&mut self) {
        self.select_prev_card_with_prs(&std::collections::HashMap::new(), &[]);
    }

    pub fn select_prev_card_with_prs(
        &mut self,
        branch_prs: &std::collections::HashMap<String, BranchPrInfo>,
        worktrees: &[crate::external::WorktreeInfo],
    ) {
        if let Some(status) = TaskStatus::from_column_index(self.selected_column) {
            let count = self.tasks_in_column_with_prs(status, branch_prs, worktrees).len();
            if count > 0 {
                let current = self.selected_card_per_column[self.selected_column];
                self.selected_card_per_column[self.selected_column] = if current == 0 {
                    count - 1
                } else {
                    current - 1
                };
            }
        }
    }

    pub fn select_next_column(&mut self) {
        self.selected_column = (self.selected_column + 1) % NUM_VISIBLE_COLUMNS;
    }

    pub fn select_prev_column(&mut self) {
        self.selected_column = if self.selected_column == 0 {
            NUM_VISIBLE_COLUMNS - 1
        } else {
            self.selected_column - 1
        };
    }
}

impl Default for TasksState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(status: TaskStatus) -> Task {
        Task {
            id: "test-id".to_string(),
            project_id: "test-project".to_string(),
            title: "Test Task".to_string(),
            description: None,
            status,
            parent_workspace_id: None,
            shared_task_id: None,
            linear_issue_id: None,
            linear_url: None,
            linear_labels: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
            has_in_progress_attempt: false,
            last_attempt_failed: false,
            executor: String::new(),
            pr_url: None,
            pr_status: None,
            pr_is_draft: None,
            pr_review_decision: None,
            pr_checks_status: None,
            pr_has_conflicts: None,
        }
    }

    #[test]
    fn test_effective_status_no_pr() {
        let task = make_task(TaskStatus::Inprogress);
        assert_eq!(task.effective_status(), TaskStatus::Inprogress);
    }

    #[test]
    fn test_effective_status_pr_open() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("open".to_string());
        task.pr_is_draft = Some(false);
        // Open non-draft PR should move to In Review
        assert_eq!(task.effective_status(), TaskStatus::Inreview);
    }

    #[test]
    fn test_effective_status_pr_draft() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("open".to_string());
        task.pr_is_draft = Some(true);
        // Draft PR should stay in current status
        assert_eq!(task.effective_status(), TaskStatus::Inprogress);
    }

    #[test]
    fn test_effective_status_pr_merged() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("merged".to_string());
        // Merged PR should move to Done
        assert_eq!(task.effective_status(), TaskStatus::Done);
    }

    #[test]
    fn test_effective_status_pr_closed() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("closed".to_string());
        // Closed PR should move to Cancelled
        assert_eq!(task.effective_status(), TaskStatus::Cancelled);
    }

    #[test]
    fn test_tasks_in_column_with_pr_transitions() {
        let mut state = TasksState::new();

        let mut task1 = make_task(TaskStatus::Inprogress);
        task1.id = "task1".to_string();

        let mut task2 = make_task(TaskStatus::Inprogress);
        task2.id = "task2".to_string();
        task2.pr_status = Some("open".to_string());
        task2.pr_is_draft = Some(false);

        let mut task3 = make_task(TaskStatus::Inprogress);
        task3.id = "task3".to_string();
        task3.pr_status = Some("merged".to_string());

        state.set_tasks(vec![task1, task2, task3]);

        // In Progress column should only have task1 (no PR)
        let in_progress = state.tasks_in_column(TaskStatus::Inprogress);
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].id, "task1");

        // In Review column should have task2 (open PR)
        let in_review = state.tasks_in_column(TaskStatus::Inreview);
        assert_eq!(in_review.len(), 1);
        assert_eq!(in_review[0].id, "task2");

        // Done column should have task3 (merged PR)
        let done = state.tasks_in_column(TaskStatus::Done);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, "task3");
    }
}
