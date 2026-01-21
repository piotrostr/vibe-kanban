use super::tasks::Task;

/// Result item that can be searched
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub result_type: SearchResultType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResultType {
    Task,
}

impl SearchResult {
    pub fn from_task(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            description: task.description.clone(),
            result_type: SearchResultType::Task,
        }
    }
}

pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub all_tasks: Vec<Task>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            all_tasks: Vec::new(),
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<Task>) {
        self.all_tasks = tasks;
        self.update_results();
    }

    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.update_results();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_results();
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.results.clear();
        self.selected_index = 0;
    }

    fn update_results(&mut self) {
        if self.query.is_empty() {
            // Show all tasks when query is empty
            self.results = self
                .all_tasks
                .iter()
                .map(SearchResult::from_task)
                .collect();
        } else {
            let query_lower = self.query.to_lowercase();
            self.results = self
                .all_tasks
                .iter()
                .filter(|task| {
                    task.title.to_lowercase().contains(&query_lower)
                        || task
                            .description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query_lower))
                })
                .map(SearchResult::from_task)
                .collect();
        }

        // Reset selection if out of bounds
        if self.selected_index >= self.results.len() {
            self.selected_index = 0;
        }
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.results.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected_index)
    }

    pub fn selected_task(&self) -> Option<&Task> {
        let result = self.selected_result()?;
        self.all_tasks.iter().find(|t| t.id == result.id)
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
