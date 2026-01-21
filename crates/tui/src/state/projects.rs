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
}

impl ProjectsState {
    pub fn new() -> Self {
        Self {
            projects: Vec::new(),
        }
    }
}

impl Default for ProjectsState {
    fn default() -> Self {
        Self::new()
    }
}
