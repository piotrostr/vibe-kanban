use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub task_id: String,
    pub container_ref: Option<String>,
    pub branch: String,
    pub agent_working_dir: Option<String>,
    pub setup_completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub workspace_id: String,
    pub executor: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProcessStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
    Approval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProcess {
    pub id: String,
    pub session_id: String,
    pub status: ExecutionProcessStatus,
    pub exit_code: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct AttemptsState {
    pub workspaces: Vec<Workspace>,
    pub selected_workspace_index: usize,
    pub current_session: Option<Session>,
    pub processes: Vec<ExecutionProcess>,
    pub chat_input: String,
    pub chat_input_active: bool,
}

impl AttemptsState {
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
            selected_workspace_index: 0,
            current_session: None,
            processes: Vec::new(),
            chat_input: String::new(),
            chat_input_active: false,
        }
    }

    pub fn set_workspaces(&mut self, workspaces: Vec<Workspace>) {
        self.workspaces = workspaces;
        self.selected_workspace_index = 0;
    }

    pub fn selected_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.selected_workspace_index)
    }

    pub fn select_next(&mut self) {
        if !self.workspaces.is_empty() {
            self.selected_workspace_index =
                (self.selected_workspace_index + 1) % self.workspaces.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.workspaces.is_empty() {
            self.selected_workspace_index = if self.selected_workspace_index == 0 {
                self.workspaces.len() - 1
            } else {
                self.selected_workspace_index - 1
            };
        }
    }
}

impl Default for AttemptsState {
    fn default() -> Self {
        Self::new()
    }
}
