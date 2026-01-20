use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Constraint, Direction, Layout};
use tokio::task::JoinHandle;

use crate::api::{
    create_task_channel, ApiClient, CreateTask, TaskStreamConnection, TaskUpdateReceiver,
    UpdateTask,
};
use crate::external::{
    attach_zellij_foreground, edit_markdown, launch_zellij_claude_foreground, list_sessions,
    list_worktrees, session_name_for_branch,
};
use crate::input::{extract_key_event, key_to_action, Action, EventStream};
use crate::state::{AppState, Modal, View};
use crate::terminal::Terminal;
use crate::ui::{
    render_footer, render_header, render_help_modal, render_kanban_board, render_project_list,
    render_sessions, render_task_detail_with_actions, render_worktrees,
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
                View::Worktrees => {
                    render_worktrees(frame, chunks[1], &self.state.worktrees);
                }
                View::Sessions => {
                    render_sessions(frame, chunks[1], &self.state.sessions);
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
        let Some(action) = key_to_action(key, self.state.view, in_modal, false) else {
            return Ok(());
        };

        // Handle modal-specific actions
        if in_modal {
            if let Action::Back = action {
                self.state.modal = None;
            }
            return Ok(());
        }

        // Handle regular actions
        match action {
            Action::Quit => {
                self.state.should_quit = true;
            }
            Action::Back => {
                self.handle_back();
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
                self.handle_select(terminal).await?;
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
            Action::ShowWorktrees => {
                self.handle_show_worktrees().await?;
            }
            Action::CreateWorktree => {
                // TODO: Implement worktree creation modal
            }
            Action::SwitchWorktree => {
                // TODO: Implement worktree switching
            }
            Action::ShowSessions => {
                self.handle_show_sessions().await?;
            }
            Action::LaunchSession => {
                self.handle_launch_session(terminal)?;
            }
            Action::AttachSession => {
                self.handle_attach_session(terminal)?;
            }
            Action::KillSession => {
                self.handle_kill_session()?;
            }
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
            View::Worktrees => {
                self.state.worktrees.select_prev();
            }
            View::Sessions => {
                self.state.sessions.select_prev();
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
            View::Worktrees => {
                self.state.worktrees.select_next();
            }
            View::Sessions => {
                self.state.sessions.select_next();
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

    async fn handle_select(&mut self, terminal: &mut Terminal) -> Result<()> {
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
                // Launch session for task
                self.handle_launch_session(terminal)?;
            }
            View::Worktrees => {
                // Launch session in selected worktree
                self.handle_launch_session(terminal)?;
            }
            View::Sessions => {
                // Attach to selected session
                self.handle_attach_session(terminal)?;
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
            View::Kanban | View::TaskDetail => {
                if let Some(project_id) = &self.state.selected_project_id {
                    let tasks = self.api.get_tasks(project_id).await?;
                    self.state.tasks.set_tasks(tasks);
                }
            }
            View::Worktrees => {
                self.load_worktrees();
            }
            View::Sessions => {
                self.load_sessions();
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

    // Worktree and session handlers

    async fn handle_show_worktrees(&mut self) -> Result<()> {
        self.load_worktrees();
        self.state.view = View::Worktrees;
        Ok(())
    }

    fn load_worktrees(&mut self) {
        self.state.worktrees.loading = true;
        self.state.worktrees.error = None;

        match list_worktrees() {
            Ok(worktrees) => {
                self.state.worktrees.set_worktrees(worktrees);
                self.state.worktrees.loading = false;
            }
            Err(e) => {
                self.state.worktrees.error = Some(e.to_string());
                self.state.worktrees.loading = false;
            }
        }
    }

    async fn handle_show_sessions(&mut self) -> Result<()> {
        self.load_sessions();
        self.state.view = View::Sessions;
        Ok(())
    }

    fn load_sessions(&mut self) {
        self.state.sessions.loading = true;
        self.state.sessions.error = None;

        match list_sessions() {
            Ok(sessions) => {
                self.state.sessions.set_sessions(sessions);
                self.state.sessions.loading = false;
            }
            Err(e) => {
                self.state.sessions.error = Some(e.to_string());
                self.state.sessions.loading = false;
            }
        }
    }

    fn handle_launch_session(&mut self, terminal: &mut Terminal) -> Result<()> {
        // Ensure worktrees are loaded
        if self.state.worktrees.worktrees.is_empty() {
            self.load_worktrees();
        }

        // Get the worktree to launch in
        let worktree = match self.state.view {
            View::Worktrees => self.state.worktrees.selected(),
            View::Kanban | View::TaskDetail => {
                // Use current worktree if available
                self.state.worktrees.worktrees.iter().find(|w| w.is_current)
            }
            _ => None,
        };

        let Some(worktree) = worktree else {
            tracing::warn!("No worktree selected for session launch");
            return Ok(());
        };

        let session_name = session_name_for_branch(&worktree.branch);
        let worktree_path = std::path::Path::new(&worktree.path);

        // Suspend TUI, run zellij in foreground, then resume TUI
        terminal.suspend()?;

        let result = launch_zellij_claude_foreground(&session_name, worktree_path);

        terminal.resume()?;

        if let Err(e) = result {
            tracing::error!("Failed to launch session: {}", e);
        } else {
            tracing::info!("Returned from session {}", session_name);
        }

        Ok(())
    }

    fn handle_attach_session(&mut self, terminal: &mut Terminal) -> Result<()> {
        let Some(session) = self.state.sessions.selected() else {
            tracing::warn!("No session selected");
            return Ok(());
        };

        let session_name = session.name.clone();

        // Suspend TUI, attach to zellij, then resume TUI
        terminal.suspend()?;

        let result = attach_zellij_foreground(&session_name);

        terminal.resume()?;

        if let Err(e) = result {
            tracing::error!("Failed to attach session: {}", e);
        } else {
            tracing::info!("Returned from session {}", session_name);
        }

        Ok(())
    }

    fn handle_kill_session(&mut self) -> Result<()> {
        let Some(session) = self.state.sessions.selected() else {
            tracing::warn!("No session selected");
            return Ok(());
        };

        if let Err(e) = crate::external::kill_session(&session.name) {
            tracing::error!("Failed to kill session: {}", e);
        } else {
            tracing::info!("Killed session {}", session.name);
            // Refresh the sessions list
            self.load_sessions();
        }

        Ok(())
    }
}
