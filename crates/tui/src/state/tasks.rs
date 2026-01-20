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
    pub const ALL: [TaskStatus; 6] = [
        TaskStatus::Backlog,
        TaskStatus::Todo,
        TaskStatus::Inprogress,
        TaskStatus::Inreview,
        TaskStatus::Done,
        TaskStatus::Cancelled,
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
            TaskStatus::Todo => 1,
            TaskStatus::Inprogress => 2,
            TaskStatus::Inreview => 3,
            TaskStatus::Done => 4,
            TaskStatus::Cancelled => 5,
        }
    }

    pub fn from_column_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(TaskStatus::Backlog),
            1 => Some(TaskStatus::Todo),
            2 => Some(TaskStatus::Inprogress),
            3 => Some(TaskStatus::Inreview),
            4 => Some(TaskStatus::Done),
            5 => Some(TaskStatus::Cancelled),
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

pub struct TasksState {
    pub tasks: Vec<Task>,
    pub selected_column: usize,
    pub selected_card_per_column: [usize; 6],
    pub loading: bool,
}

impl TasksState {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_column: 1, // Start on "todo"
            selected_card_per_column: [0; 6],
            loading: false,
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        // Reset card selections
        self.selected_card_per_column = [0; 6];
    }

    pub fn tasks_in_column(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.status == status)
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
        self.selected_column = (self.selected_column + 1) % 6;
    }

    pub fn select_prev_column(&mut self) {
        self.selected_column = if self.selected_column == 0 {
            5
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
