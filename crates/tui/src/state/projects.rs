use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub dev_script: Option<String>,
    pub dev_script_working_dir: Option<String>,
    pub default_agent_working_dir: Option<String>,
    pub remote_project_id: Option<String>,
}

pub struct ProjectsState {
    pub projects: Vec<Project>,
    pub selected_index: usize,
    pub loading: bool,
}

impl ProjectsState {
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
            selected_index: 0,
            loading: false,
        }
    }

    pub fn set_projects(&mut self, projects: Vec<Project>) {
        self.projects = projects;
        self.selected_index = 0;
    }

    pub fn selected(&self) -> Option<&Project> {
        self.projects.get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        if !self.projects.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.projects.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.projects.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.projects.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
}

impl Default for ProjectsState {
    fn default() -> Self {
        Self::new()
    }
}
