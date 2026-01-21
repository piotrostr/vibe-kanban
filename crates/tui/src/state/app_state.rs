use super::{LogsState, ProjectsState, SessionsState, TasksState, WorktreesState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Projects,
    Kanban,
    TaskDetail,
    Worktrees,
    Sessions,
    Logs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    ProjectList,
    KanbanColumn(usize),
    KanbanCard { column: usize, card: usize },
    TaskPanel,
    SearchBar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Help,
    CreateTask,
    DeleteConfirm(String), // task_id
}

pub struct AppState {
    pub view: View,
    pub focus: Focus,
    pub modal: Option<Modal>,

    pub projects: ProjectsState,
    pub tasks: TasksState,
    pub worktrees: WorktreesState,
    pub sessions: SessionsState,
    pub logs: LogsState,

    pub selected_project_id: Option<String>,
    pub selected_task_id: Option<String>,

    // Search state
    pub search_active: bool,
    pub search_query: String,

    pub backend_connected: bool,
    pub should_quit: bool,

    // Animation state for activity spinners
    pub animation_frame: u8,

    // Linear integration
    pub linear_api_key_available: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            view: View::Projects,
            focus: Focus::ProjectList,
            modal: None,

            projects: ProjectsState::new(),
            tasks: TasksState::new(),
            worktrees: WorktreesState::new(),
            sessions: SessionsState::new(),
            logs: LogsState::new(),

            selected_project_id: None,
            selected_task_id: None,

            search_active: false,
            search_query: String::new(),

            backend_connected: false,
            should_quit: false,

            animation_frame: 0,

            linear_api_key_available: false,
        }
    }

    /// Advance the animation frame counter (wraps at 4 for spinner animation)
    pub fn tick_animation(&mut self) {
        self.animation_frame = (self.animation_frame + 1) % 4;
    }

    /// Get the current spinner character based on animation frame
    pub fn spinner_char(&self) -> char {
        const SPINNER: [char; 4] = ['|', '/', '-', '\\'];
        SPINNER[self.animation_frame as usize]
    }

    pub fn select_project(&mut self, project_id: String) {
        self.selected_project_id = Some(project_id);
        self.view = View::Kanban;
        self.focus = Focus::KanbanColumn(1); // Start on "todo" column
    }

    pub fn back(&mut self) {
        match self.view {
            View::Projects => {
                self.should_quit = true;
            }
            View::Kanban => {
                self.selected_project_id = None;
                self.view = View::Projects;
                self.focus = Focus::ProjectList;
            }
            View::TaskDetail => {
                self.selected_task_id = None;
                self.view = View::Kanban;
                self.focus = Focus::KanbanColumn(1);
            }
            View::Worktrees => {
                self.view = View::Kanban;
                self.focus = Focus::KanbanColumn(1);
            }
            View::Sessions => {
                self.view = View::Kanban;
                self.focus = Focus::KanbanColumn(1);
            }
            View::Logs => {
                self.view = View::Kanban;
                self.focus = Focus::KanbanColumn(1);
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive the env var name for the Linear API key from a project name.
/// E.g., "vibe-kanban" -> "VIBE_KANBAN_LINEAR_API_KEY"
pub fn linear_env_var_name(project_name: &str) -> String {
    let normalized: String = project_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("{}_LINEAR_API_KEY", normalized)
}

/// Check if the Linear API key env var is set for the given project name
pub fn check_linear_api_key(project_name: &str) -> bool {
    let env_var = linear_env_var_name(project_name);
    std::env::var(&env_var).is_ok()
}
