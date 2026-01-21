use super::{ProjectsState, SessionsState, TasksState, WorktreesState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Projects,
    Kanban,
    TaskDetail,
    Worktrees,
    Sessions,
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

    pub selected_project_id: Option<String>,
    pub selected_task_id: Option<String>,

    // Search state
    pub search_active: bool,
    pub search_query: String,

    pub backend_connected: bool,
    pub should_quit: bool,
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

            selected_project_id: None,
            selected_task_id: None,

            search_active: false,
            search_query: String::new(),

            backend_connected: false,
            should_quit: false,
        }
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
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
