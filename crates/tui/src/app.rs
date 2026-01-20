use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Constraint, Direction, Layout};
use tokio::task::JoinHandle;

use crate::api::{
    create_task_channel, ApiClient, CreateSession, CreateTask, CreateTaskAttempt,
    ExecutorProfileId, FollowUpRequest, TaskStreamConnection, TaskUpdateReceiver, UpdateTask,
    WorkspaceRepoInput,
};
use crate::external::edit_markdown;
use crate::input::{extract_key_event, key_to_action, Action, EventStream};
use crate::state::{AppState, Modal, View};
use crate::terminal::Terminal;
use crate::ui::{
    render_attempt_chat, render_footer, render_header, render_help_modal, render_kanban_board,
    render_project_list, render_task_detail_with_actions,
};

pub struct App {
    state: AppState,
    api: ApiClient,
    events: EventStream,
    port: u16,
    ws_task: Option<JoinHandle<()>>,
    task_receiver: Option<TaskUpdateReceiver>,
}

impl App {
    pub async fn new(port: u16) -> Result<Self> {
        let api = ApiClient::new(port);
        let mut state = AppState::new();

        // Verify connection
        api.health_check().await?;
        state.backend_connected = true;

        // Load initial data
        let projects = api.get_projects().await?;
        state.projects.set_projects(projects);

        Ok(Self {
            state,
            api,
            events: EventStream::new(),
            port,
            ws_task: None,
            task_receiver: None,
        })
    }

    pub async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        loop {
            // Check for WebSocket updates
            self.check_ws_updates();

            // Render
            self.render(terminal)?;

            // Handle events
            if let Some(event) = self.events.next().await? {
                self.handle_event(event, terminal).await?;
            }

            if self.state.should_quit {
                break;
            }
        }

        // Cleanup WebSocket task
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }

        Ok(())
    }

    fn check_ws_updates(&mut self) {
        if let Some(ref mut receiver) = self.task_receiver {
            // Non-blocking check for updates
            while let Ok(tasks) = receiver.try_recv() {
                self.state.tasks.set_tasks(tasks);
            }
        }
    }

    fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Header
                    Constraint::Min(0),    // Main content
                    Constraint::Length(2), // Footer
                ])
                .split(frame.area());

            render_header(frame, chunks[0], &self.state);

            match self.state.view {
                View::Projects => {
                    render_project_list(frame, chunks[1], &self.state.projects);
                }
                View::Kanban => {
                    render_kanban_board(frame, chunks[1], &self.state.tasks);
                }
                View::TaskDetail => {
                    // Find the selected task
                    if let Some(task_id) = &self.state.selected_task_id {
                        if let Some(task) = self.state.tasks.tasks.iter().find(|t| &t.id == task_id)
                        {
                            render_task_detail_with_actions(frame, chunks[1], task);
                        }
                    }
                }
                View::AttemptChat => {
                    if let Some(task_id) = &self.state.selected_task_id {
                        if let Some(task) = self.state.tasks.tasks.iter().find(|t| &t.id == task_id)
                        {
                            render_attempt_chat(frame, chunks[1], task, &self.state.attempts);
                        }
                    }
                }
            }

            render_footer(frame, chunks[2], &self.state);

            // Render modal if present
            if let Some(Modal::Help) = &self.state.modal {
                render_help_modal(frame, frame.area());
            }
        })?;

        Ok(())
    }

    async fn handle_event(&mut self, event: Event, terminal: &mut Terminal) -> Result<()> {
        let Some(key) = extract_key_event(event) else {
            return Ok(());
        };

        let in_modal = self.state.modal.is_some();
        let chat_input_active = self.state.attempts.chat_input_active;
        let Some(action) = key_to_action(key, self.state.view, in_modal, chat_input_active) else {
            return Ok(());
        };

        // Handle modal-specific actions
        if in_modal {
            match action {
                Action::Back => {
                    self.state.modal = None;
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle regular actions
        match action {
            Action::Quit => {
                self.state.should_quit = true;
            }
            Action::Back => {
                // If chat input is active, just deactivate it
                if self.state.attempts.chat_input_active {
                    self.state.attempts.chat_input_active = false;
                } else {
                    self.handle_back();
                }
            }
            Action::ShowHelp => {
                self.state.modal = Some(Modal::Help);
            }
            Action::Up => {
                self.handle_up();
            }
            Action::Down => {
                self.handle_down();
            }
            Action::Left => {
                self.handle_left();
            }
            Action::Right => {
                self.handle_right();
            }
            Action::Select => {
                self.handle_select().await?;
            }
            Action::Refresh => {
                self.refresh().await?;
            }
            Action::EditTask => {
                self.handle_edit_task(terminal).await?;
            }
            Action::CreateTask => {
                self.handle_create_task(terminal).await?;
            }
            Action::DeleteTask => {
                self.handle_delete_task().await?;
            }
            Action::StartAttempt => {
                self.handle_start_attempt().await?;
            }
            Action::StopAttempt => {
                // TODO: Implement stop attempt
            }
            Action::OpenAttemptChat => {
                self.handle_open_attempt_chat().await?;
            }
            Action::FocusInput => {
                if self.state.view == View::AttemptChat {
                    self.state.attempts.chat_input_active = true;
                }
            }
            Action::SendMessage => {
                self.handle_send_message().await?;
            }
            Action::TypeChar(c) => {
                if self.state.attempts.chat_input_active {
                    self.state.attempts.chat_input.push(c);
                }
            }
            Action::Backspace => {
                if self.state.attempts.chat_input_active {
                    self.state.attempts.chat_input.pop();
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_back(&mut self) {
        // Stop WebSocket when leaving kanban view
        if self.state.view == View::Kanban {
            self.stop_ws_stream();
        }
        self.state.back();
    }

    fn handle_up(&mut self) {
        match self.state.view {
            View::Projects => {
                self.state.projects.select_prev();
            }
            View::Kanban => {
                self.state.tasks.select_prev_card();
            }
            View::TaskDetail => {
                // TODO: Scroll
            }
            View::AttemptChat => {
                self.state.attempts.select_prev();
            }
        }
    }

    fn handle_down(&mut self) {
        match self.state.view {
            View::Projects => {
                self.state.projects.select_next();
            }
            View::Kanban => {
                self.state.tasks.select_next_card();
            }
            View::TaskDetail => {
                // TODO: Scroll
            }
            View::AttemptChat => {
                self.state.attempts.select_next();
            }
        }
    }

    fn handle_left(&mut self) {
        if self.state.view == View::Kanban {
            self.state.tasks.select_prev_column();
        }
    }

    fn handle_right(&mut self) {
        if self.state.view == View::Kanban {
            self.state.tasks.select_next_column();
        }
    }

    async fn handle_select(&mut self) -> Result<()> {
        match self.state.view {
            View::Projects => {
                if let Some(project) = self.state.projects.selected() {
                    let project_id = project.id.clone();

                    // Load tasks for this project
                    self.state.tasks.loading = true;
                    let tasks = self.api.get_tasks(&project_id).await?;
                    self.state.tasks.set_tasks(tasks);
                    self.state.tasks.loading = false;

                    // Start WebSocket stream for real-time updates
                    self.start_ws_stream(&project_id);

                    self.state.select_project(project_id);
                }
            }
            View::Kanban => {
                if let Some(task) = self.state.tasks.selected_task() {
                    self.state.selected_task_id = Some(task.id.clone());
                    self.state.view = View::TaskDetail;
                }
            }
            View::TaskDetail => {
                // Open attempt chat view
                self.handle_open_attempt_chat().await?;
            }
            View::AttemptChat => {
                // Focus input when selecting in attempt chat
                self.state.attempts.chat_input_active = true;
            }
        }

        Ok(())
    }

    fn start_ws_stream(&mut self, project_id: &str) {
        // Stop any existing stream
        self.stop_ws_stream();

        let (sender, receiver) = create_task_channel();
        self.task_receiver = Some(receiver);

        let base_url = format!("http://127.0.0.1:{}", self.port);
        let project_id = project_id.to_string();

        let task = tokio::spawn(async move {
            loop {
                match TaskStreamConnection::connect(&base_url, &project_id, sender.clone()).await {
                    Ok(()) => {
                        tracing::info!("WebSocket connection closed normally");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("WebSocket connection error: {}, reconnecting...", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
        });

        self.ws_task = Some(task);
    }

    fn stop_ws_stream(&mut self) {
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        self.task_receiver = None;
    }

    async fn refresh(&mut self) -> Result<()> {
        match self.state.view {
            View::Projects => {
                let projects = self.api.get_projects().await?;
                self.state.projects.set_projects(projects);
            }
            View::Kanban => {
                if let Some(project_id) = &self.state.selected_project_id {
                    let tasks = self.api.get_tasks(project_id).await?;
                    self.state.tasks.set_tasks(tasks);
                }
            }
            View::TaskDetail => {
                // Refresh by reloading tasks
                if let Some(project_id) = &self.state.selected_project_id {
                    let tasks = self.api.get_tasks(project_id).await?;
                    self.state.tasks.set_tasks(tasks);
                }
            }
            View::AttemptChat => {
                // Refresh attempts
                self.load_attempts().await?;
            }
        }

        Ok(())
    }

    async fn handle_edit_task(&mut self, terminal: &mut Terminal) -> Result<()> {
        // Get the selected task
        let task_id = match self.state.view {
            View::TaskDetail => self.state.selected_task_id.clone(),
            View::Kanban => self.state.tasks.selected_task().map(|t| t.id.clone()),
            _ => None,
        };

        let Some(task_id) = task_id else {
            return Ok(());
        };

        // Find the task
        let task = self
            .state
            .tasks
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .cloned();

        let Some(task) = task else {
            return Ok(());
        };

        // Suspend terminal for editor
        terminal.suspend()?;

        // Edit in external editor
        let content = format!(
            "# {}\n\n{}",
            task.title,
            task.description.as_deref().unwrap_or("")
        );

        let edited = edit_markdown(&content);

        // Resume terminal
        terminal.resume()?;

        // Process the edit
        if let Ok(Some(new_content)) = edited {
            // Parse the edited content - first line is title, rest is description
            let mut lines = new_content.lines();
            let title_line = lines.next().unwrap_or(&task.title);
            let title = title_line.trim_start_matches('#').trim().to_string();
            let description: String = lines.collect::<Vec<_>>().join("\n").trim().to_string();

            let update = UpdateTask {
                title: Some(title),
                description: Some(if description.is_empty() {
                    task.description.clone().unwrap_or_default()
                } else {
                    description
                }),
                status: None,
                sync_to_linear: false,
            };

            self.api.update_task(&task_id, update).await?;

            // Refresh to get updated data
            self.refresh().await?;
        }

        Ok(())
    }

    async fn handle_create_task(&mut self, terminal: &mut Terminal) -> Result<()> {
        let Some(project_id) = self.state.selected_project_id.clone() else {
            return Ok(());
        };

        // Suspend terminal for editor
        terminal.suspend()?;

        // Edit new task in editor
        let content = "# New Task\n\nDescription here...";
        let edited = edit_markdown(content);

        // Resume terminal
        terminal.resume()?;

        // Process the edit
        if let Ok(Some(new_content)) = edited {
            // Parse the edited content
            let mut lines = new_content.lines();
            let title_line = lines.next().unwrap_or("New Task");
            let title = title_line.trim_start_matches('#').trim().to_string();
            let description: String = lines.collect::<Vec<_>>().join("\n").trim().to_string();

            if title.is_empty() || title == "New Task" {
                return Ok(()); // Cancelled
            }

            let create = CreateTask {
                project_id,
                title,
                description: if description.is_empty() || description == "Description here..." {
                    None
                } else {
                    Some(description)
                },
                status: None,
            };

            self.api.create_task(create).await?;

            // Refresh to get updated data
            self.refresh().await?;
        }

        Ok(())
    }

    async fn handle_delete_task(&mut self) -> Result<()> {
        // Get the selected task
        let task_id = match self.state.view {
            View::TaskDetail => self.state.selected_task_id.clone(),
            View::Kanban => self.state.tasks.selected_task().map(|t| t.id.clone()),
            _ => None,
        };

        let Some(task_id) = task_id else {
            return Ok(());
        };

        // Delete the task
        self.api.delete_task(&task_id).await?;

        // Go back if we were in task detail view
        if self.state.view == View::TaskDetail {
            self.state.selected_task_id = None;
            self.state.view = View::Kanban;
        }

        // Refresh to get updated data
        self.refresh().await?;

        Ok(())
    }

    async fn handle_start_attempt(&mut self) -> Result<()> {
        let Some(task_id) = self.state.selected_task_id.clone() else {
            return Ok(());
        };

        // For now, we need project repos to create an attempt
        // This is a simplified version - in reality we'd need to fetch repos
        // and let the user select which ones to include
        let create = CreateTaskAttempt {
            task_id,
            executor_profile_id: ExecutorProfileId {
                executor: "CLAUDE_CODE".to_string(),
                variant: None,
            },
            repos: vec![], // Empty for now - the backend should handle this
        };

        match self.api.create_task_attempt(create).await {
            Ok(workspace) => {
                tracing::info!("Created attempt: {}", workspace.id);
                // Refresh attempts list
                self.load_attempts().await?;
                // Switch to attempt chat view
                self.state.view = View::AttemptChat;
            }
            Err(e) => {
                tracing::error!("Failed to create attempt: {}", e);
            }
        }

        Ok(())
    }

    async fn handle_open_attempt_chat(&mut self) -> Result<()> {
        // Load attempts for the current task
        self.load_attempts().await?;
        self.state.view = View::AttemptChat;
        Ok(())
    }

    async fn load_attempts(&mut self) -> Result<()> {
        let Some(task_id) = self.state.selected_task_id.clone() else {
            return Ok(());
        };

        let workspaces = self.api.get_task_attempts(&task_id).await?;
        self.state.attempts.set_workspaces(workspaces);

        // Load session for selected workspace
        if let Some(workspace) = self.state.attempts.selected_workspace() {
            let sessions = self.api.get_sessions(&workspace.id).await?;
            self.state.attempts.current_session = sessions.into_iter().next();
        }

        Ok(())
    }

    async fn handle_send_message(&mut self) -> Result<()> {
        let message = self.state.attempts.chat_input.trim().to_string();
        if message.is_empty() {
            return Ok(());
        }

        // Get or create session
        let session_id = if let Some(session) = &self.state.attempts.current_session {
            session.id.clone()
        } else {
            // Need a workspace first
            let Some(workspace) = self.state.attempts.selected_workspace() else {
                tracing::warn!("No workspace selected");
                return Ok(());
            };

            // Create a new session
            let session = self
                .api
                .create_session(CreateSession {
                    workspace_id: workspace.id.clone(),
                    executor: Some("CLAUDE_CODE".to_string()),
                })
                .await?;

            self.state.attempts.current_session = Some(session.clone());
            session.id
        };

        // Send the follow-up
        let follow_up = FollowUpRequest {
            prompt: message,
            variant: None,
        };

        match self.api.send_follow_up(&session_id, follow_up).await {
            Ok(process) => {
                tracing::info!("Started execution process: {}", process.id);
                self.state.attempts.processes.push(process);
            }
            Err(e) => {
                tracing::error!("Failed to send message: {}", e);
            }
        }

        // Clear input
        self.state.attempts.chat_input.clear();
        self.state.attempts.chat_input_active = false;

        Ok(())
    }
}
