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

    pub fn column_index(&self) -> usize {
        match self {
            TaskStatus::Backlog => 0,
            TaskStatus::Todo => 0,
            TaskStatus::Inprogress => 1,
            TaskStatus::Inreview => 2,
            TaskStatus::Done => 3,
            TaskStatus::Cancelled => 3,
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
    pub fn effective_status(&self) -> TaskStatus {
        if let Some(ref pr_status) = self.pr_status {
            match pr_status.as_str() {
                "merged" => return TaskStatus::Done,
                "closed" => return TaskStatus::Cancelled,
                "open" => {
                    if self.pr_is_draft != Some(true) {
                        return TaskStatus::Inreview;
                    }
                }
                _ => {}
            }
        }
        self.status
    }

    pub fn effective_status_with_pr(
        &self,
        branch_pr: Option<&BranchPrInfo>,
        has_worktree: bool,
    ) -> TaskStatus {
        if self.pr_status.is_some() {
            return self.effective_status();
        }

        if let Some(pr) = branch_pr {
            match pr.state.as_str() {
                "MERGED" => return TaskStatus::Done,
                "CLOSED" => return TaskStatus::Cancelled,
                "OPEN" => {
                    if !pr.is_draft {
                        return TaskStatus::Inreview;
                    }
                    if has_worktree {
                        return TaskStatus::Inprogress;
                    }
                }
                _ => {}
            }
        }

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
    pub search_filter: String,
}

impl TasksState {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_column: 0,
            selected_card_per_column: [0; NUM_VISIBLE_COLUMNS],
            search_filter: String::new(),
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        self.selected_card_per_column = [0; NUM_VISIBLE_COLUMNS];
    }

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

    pub fn select_next_card_with_prs(
        &mut self,
        branch_prs: &std::collections::HashMap<String, BranchPrInfo>,
        worktrees: &[crate::external::WorktreeInfo],
    ) {
        if let Some(status) = TaskStatus::from_column_index(self.selected_column) {
            let count = self.tasks_in_column_with_prs(status, branch_prs, worktrees).len();
            if count > 0 {
                let current = self.selected_card_per_column[self.selected_column];
                if current + 1 >= count {
                    // At the last card - move to next row
                    self.select_next_column();
                } else {
                    self.selected_card_per_column[self.selected_column] = current + 1;
                }
            } else {
                // Empty row - move to next row
                self.select_next_column();
            }
        }
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
                if current == 0 {
                    // At the first card - move to previous row and select last card
                    self.select_prev_column();
                    // Select last card in new row
                    if let Some(new_status) = TaskStatus::from_column_index(self.selected_column) {
                        let new_count =
                            self.tasks_in_column_with_prs(new_status, branch_prs, worktrees).len();
                        if new_count > 0 {
                            self.selected_card_per_column[self.selected_column] = new_count - 1;
                        }
                    }
                } else {
                    self.selected_card_per_column[self.selected_column] = current - 1;
                }
            } else {
                // Empty row - move to previous row
                self.select_prev_column();
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
        assert_eq!(task.effective_status(), TaskStatus::Inreview);
    }

    #[test]
    fn test_effective_status_pr_draft() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("open".to_string());
        task.pr_is_draft = Some(true);
        assert_eq!(task.effective_status(), TaskStatus::Inprogress);
    }

    #[test]
    fn test_effective_status_pr_merged() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("merged".to_string());
        assert_eq!(task.effective_status(), TaskStatus::Done);
    }

    #[test]
    fn test_effective_status_pr_closed() {
        let mut task = make_task(TaskStatus::Inprogress);
        task.pr_url = Some("https://github.com/org/repo/pull/1".to_string());
        task.pr_status = Some("closed".to_string());
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

        let empty_prs = std::collections::HashMap::new();
        let empty_wt: Vec<crate::external::WorktreeInfo> = vec![];

        let in_progress = state.tasks_in_column_with_prs(TaskStatus::Inprogress, &empty_prs, &empty_wt);
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].id, "task1");

        let in_review = state.tasks_in_column_with_prs(TaskStatus::Inreview, &empty_prs, &empty_wt);
        assert_eq!(in_review.len(), 1);
        assert_eq!(in_review[0].id, "task2");

        let done = state.tasks_in_column_with_prs(TaskStatus::Done, &empty_prs, &empty_wt);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, "task3");
    }
}
