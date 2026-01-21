use crate::external::ZellijSession;

pub struct SessionsState {
    pub sessions: Vec<ZellijSession>,
    pub selected_index: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl SessionsState {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
            loading: false,
            error: None,
        }
    }

    pub fn set_sessions(&mut self, sessions: Vec<ZellijSession>) {
        self.sessions = sessions;
        self.error = None;
        if self.selected_index >= self.sessions.len() {
            self.selected_index = self.sessions.len().saturating_sub(1);
        }
    }

    pub fn selected(&self) -> Option<&ZellijSession> {
        self.sessions.get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.sessions.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.sessions.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.sessions.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn session_for_branch(&self, branch: &str) -> Option<&ZellijSession> {
        let sanitized = crate::external::session_name_for_branch(branch);
        self.sessions.iter().find(|s| s.name == sanitized)
    }
}

impl Default for SessionsState {
    fn default() -> Self {
        Self::new()
    }
}
