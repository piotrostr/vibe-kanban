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
        let column_index = status.column_index();
        self.tasks
            .iter()
            .filter(|t| t.status.column_index() == column_index)
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
        let status = TaskStatus::from_column_index(self.selected_column)?;
        let tasks = self.tasks_in_column(status);
        let card_index = self.selected_card_per_column[self.selected_column];
        tasks.get(card_index).copied()
    }

    pub fn select_next_card(&mut self) {
        if let Some(status) = TaskStatus::from_column_index(self.selected_column) {
            let count = self.tasks_in_column(status).len();
            if count > 0 {
                let current = self.selected_card_per_column[self.selected_column];
                self.selected_card_per_column[self.selected_column] = (current + 1) % count;
            }
        }
    }

    pub fn select_prev_card(&mut self) {
        if let Some(status) = TaskStatus::from_column_index(self.selected_column) {
            let count = self.tasks_in_column(status).len();
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
