use super::{LogsState, ProjectsState, SearchState, SessionsState, TasksState, WorktreesState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Projects,
    Kanban,
    TaskDetail,
    Worktrees,
    Sessions,
    Logs,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Help,
}

pub struct AppState {
    pub view: View,
    pub modal: Option<Modal>,

    pub projects: ProjectsState,
    pub tasks: TasksState,
    pub worktrees: WorktreesState,
    pub sessions: SessionsState,
    pub logs: LogsState,
    pub search: SearchState,

    pub selected_project_id: Option<String>,
    pub selected_task_id: Option<String>,

    pub search_active: bool,
    pub search_query: String,

    pub backend_connected: bool,
    pub should_quit: bool,

    pub animation_frame: u8,

    pub linear_api_key_available: bool,

    /// When true, logs are shown as an overlay on top of the current view
    pub logs_overlay_visible: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            view: View::Projects,
            modal: None,

            projects: ProjectsState::new(),
            tasks: TasksState::new(),
            worktrees: WorktreesState::new(),
            sessions: SessionsState::new(),
            logs: LogsState::new(),
            search: SearchState::new(),

            selected_project_id: None,
            selected_task_id: None,

            search_active: false,
            search_query: String::new(),

            backend_connected: false,
            should_quit: false,

            animation_frame: 0,

            linear_api_key_available: false,

            logs_overlay_visible: false,
        }
    }

    pub fn tick_animation(&mut self) {
        self.animation_frame = (self.animation_frame + 1) % 4;
    }

    pub fn spinner_char(&self) -> char {
        const SPINNER: [char; 4] = ['|', '/', '-', '\\'];
        SPINNER[self.animation_frame as usize]
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

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

pub fn check_linear_api_key(project_name: &str) -> bool {
    let env_var = linear_env_var_name(project_name);
    std::env::var(&env_var).is_ok()
}
