use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Constraint, Direction, Layout};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::api::{
    create_task_channel, ApiClient, CreateTask, CreateTaskAttemptRepoRequest,
    CreateTaskAttemptRequest, TaskStreamConnection, TaskUpdateReceiver, UpdateTask,
};
use crate::external::{
    attach_zellij_foreground, edit_markdown, launch_zellij_claude_in_worktree,
    launch_zellij_claude_in_worktree_with_context, list_prs, list_sessions_with_status,
    list_worktrees, select_pr_with_fzf, WorktreeInfo, ZellijSession,
};
use crate::input::{extract_key_event, key_to_action, Action, EventStream};
use crate::state::{check_linear_api_key, AppState, Modal, View};
use crate::terminal::Terminal;
use crate::ui::{
    render_footer, render_header, render_help_modal, render_kanban_board, render_project_list,
    render_sessions, render_task_detail_with_actions, render_worktrees,
};

type WorktreeResult = Result<Vec<WorktreeInfo>, String>;
type SessionResult = Result<Vec<ZellijSession>, String>;

pub struct App {
    state: AppState,
    api: ApiClient,
    events: EventStream,
    port: u16,
    ws_task: Option<JoinHandle<()>>,
    task_receiver: Option<TaskUpdateReceiver>,
    last_session_poll: std::time::Instant,
    last_animation_tick: std::time::Instant,
    // Background loading channels
    worktree_receiver: mpsc::Receiver<WorktreeResult>,
    worktree_sender: mpsc::Sender<WorktreeResult>,
    session_receiver: mpsc::Receiver<SessionResult>,
    session_sender: mpsc::Sender<SessionResult>,
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

        // Create background loading channels
        let (worktree_sender, worktree_receiver) = mpsc::channel(4);
        let (session_sender, session_receiver) = mpsc::channel(4);

        // Mark as loading immediately so UI shows loading state
        state.worktrees.loading = true;
        state.sessions.loading = true;

        // Spawn immediate background load for worktrees
        let wt_sender = worktree_sender.clone();
        tokio::task::spawn_blocking(move || {
            let result = list_worktrees().map_err(|e| e.to_string());
            let _ = wt_sender.blocking_send(result);
        });

        // Spawn immediate background load for sessions
        let sess_sender = session_sender.clone();
        tokio::task::spawn_blocking(move || {
            let result = list_sessions_with_status().map_err(|e| e.to_string());
            let _ = sess_sender.blocking_send(result);
        });

        Ok(Self {
            state,
            api,
            events: EventStream::new(),
            port,
            ws_task: None,
            task_receiver: None,
            last_session_poll: std::time::Instant::now(),
            last_animation_tick: std::time::Instant::now(),
            worktree_receiver,
            worktree_sender,
            session_receiver,
            session_sender,
        })
    }

    pub async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        // Poll session status every 5 seconds
        const SESSION_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
        // Tick animation every 250ms for smooth spinner
        const ANIMATION_TICK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

        loop {
            // Check for WebSocket updates
            self.check_ws_updates();

            // Check for background load results (worktrees, sessions)
            self.check_background_loads();

            // Poll session status periodically (non-blocking background refresh)
            if self.last_session_poll.elapsed() >= SESSION_POLL_INTERVAL {
                self.poll_sessions_async();
                self.last_session_poll = std::time::Instant::now();
            }

            // Tick animation for spinners
            if self.last_animation_tick.elapsed() >= ANIMATION_TICK_INTERVAL {
                self.state.tick_animation();
                self.last_animation_tick = std::time::Instant::now();
            }

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

    fn check_background_loads(&mut self) {
        // Non-blocking check for worktree results
        while let Ok(result) = self.worktree_receiver.try_recv() {
            match result {
                Ok(worktrees) => {
                    self.state.worktrees.set_worktrees(worktrees);
                    self.state.worktrees.loading = false;
                    self.state.worktrees.error = None;
                }
                Err(e) => {
                    self.state.worktrees.error = Some(e);
                    self.state.worktrees.loading = false;
                }
            }
        }

        // Non-blocking check for session results
        while let Ok(result) = self.session_receiver.try_recv() {
            match result {
                Ok(sessions) => {
                    self.state.sessions.set_sessions(sessions);
                    self.state.sessions.loading = false;
                    self.state.sessions.error = None;
                }
                Err(e) => {
                    self.state.sessions.error = Some(e);
                    self.state.sessions.loading = false;
                }
            }
        }
    }

    fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // Header with ASCII logo
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
                    render_kanban_board(
                        frame,
                        chunks[1],
                        &self.state.tasks,
                        &self.state.worktrees,
                        &self.state.sessions,
                        self.state.spinner_char(),
                    );
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
        let Some(action) = key_to_action(key, self.state.view, in_modal, self.state.search_active)
        else {
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
                self.handle_launch_session(terminal, false)?;
            }
            Action::LaunchSessionPlan => {
                self.handle_launch_session(terminal, true)?;
            }
            Action::ViewPR => {
                self.handle_view_pr()?;
            }
            Action::BindPR => {
                self.handle_bind_pr(terminal).await?;
            }
            Action::AttachSession => {
                self.handle_attach_session(terminal)?;
            }
            Action::KillSession => {
                self.handle_kill_session()?;
            }

            // Search actions
            Action::StartSearch => {
                self.state.search_active = true;
            }
            Action::SearchType(c) => {
                self.state.search_query.push(c);
            }
            Action::SearchBackspace => {
                self.state.search_query.pop();
            }
            Action::SearchConfirm => {
                self.state.search_active = false;
                // Apply filter to tasks
                self.state.tasks.search_filter = self.state.search_query.clone();
            }
            Action::SearchCancel => {
                self.state.search_active = false;
                self.state.search_query.clear();
                self.state.tasks.search_filter.clear();
            }
            Action::ClearSearch => {
                self.state.search_query.clear();
                self.state.tasks.search_filter.clear();
            }

            Action::SyncLinear => {
                self.handle_sync_linear().await?;
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
                    let project_name = project.name.clone();

                    // Check if Linear API key env var is available
                    self.state.linear_api_key_available = check_linear_api_key(&project_name);

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
                self.handle_launch_session(terminal, false)?;
            }
            View::Worktrees => {
                // Launch session in selected worktree
                self.handle_launch_session(terminal, false)?;
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
        // Skip if already loading
        if self.state.worktrees.loading {
            return;
        }

        self.state.worktrees.loading = true;
        self.state.worktrees.error = None;

        // Spawn background task
        let sender = self.worktree_sender.clone();
        tokio::task::spawn_blocking(move || {
            let result = list_worktrees().map_err(|e| e.to_string());
            let _ = sender.blocking_send(result);
        });
    }

    async fn handle_show_sessions(&mut self) -> Result<()> {
        self.load_sessions();
        self.state.view = View::Sessions;
        Ok(())
    }

    fn load_sessions(&mut self) {
        // Skip if already loading
        if self.state.sessions.loading {
            return;
        }

        self.state.sessions.loading = true;
        self.state.sessions.error = None;

        // Spawn background task
        let sender = self.session_sender.clone();
        tokio::task::spawn_blocking(move || {
            let result = list_sessions_with_status().map_err(|e| e.to_string());
            let _ = sender.blocking_send(result);
        });
    }

    fn poll_sessions_async(&mut self) {
        // Spawn background task to refresh session status
        // Only if not already loading (avoid stacking requests)
        if !self.state.sessions.loading {
            let sender = self.session_sender.clone();
            tokio::task::spawn_blocking(move || {
                let result = list_sessions_with_status().map_err(|e| e.to_string());
                let _ = sender.blocking_send(result);
            });
        }
    }

    fn handle_launch_session(&mut self, terminal: &mut Terminal, plan_mode: bool) -> Result<()> {
        // Get task and derive branch name
        let task = match self.state.view {
            View::Worktrees => {
                // If in worktrees view, use selected worktree directly
                if let Some(wt) = self.state.worktrees.selected() {
                    terminal.suspend()?;
                    let result = launch_zellij_claude_in_worktree(
                        &wt.branch,
                        plan_mode,
                    );
                    terminal.resume()?;
                    if let Err(e) = result {
                        tracing::error!("Failed to launch session: {}", e);
                    }
                    return Ok(());
                }
                return Ok(());
            }
            View::Kanban | View::TaskDetail => self.state.tasks.selected_task(),
            _ => None,
        };

        let Some(task) = task else {
            tracing::warn!("No task selected for session launch");
            return Ok(());
        };

        // Create branch slug from task title
        let branch = task_title_to_branch(&task.title);

        // Build task context for fresh sessions
        let task_context = {
            let mut context = format!("Task: {}", task.title);
            if let Some(desc) = &task.description {
                if !desc.is_empty() {
                    context.push_str(&format!("\n\nDescription:\n{}", desc));
                }
            }
            context
        };

        // Suspend TUI, create worktree if needed, launch claude
        terminal.suspend()?;

        let result = launch_zellij_claude_in_worktree_with_context(
            &branch,
            &task_context,
            plan_mode,
        );

        terminal.resume()?;

        if let Err(e) = result {
            tracing::error!("Failed to launch session: {}", e);
        }

        Ok(())
    }

    fn handle_view_pr(&self) -> Result<()> {
        if let Some(task) = self.state.tasks.selected_task() {
            if let Some(pr_url) = &task.pr_url {
                if let Err(e) = open::that(pr_url) {
                    tracing::error!("Failed to open PR URL: {}", e);
                }
            } else {
                tracing::warn!("No PR URL for this task");
            }
        }
        Ok(())
    }

    async fn handle_bind_pr(&mut self, terminal: &mut Terminal) -> Result<()> {
        // Get the selected task
        let task = match self.state.view {
            View::Kanban | View::TaskDetail => self.state.tasks.selected_task().cloned(),
            _ => None,
        };

        let Some(task) = task else {
            tracing::warn!("No task selected for PR binding");
            return Ok(());
        };

        // Get project repos to find the repo_id
        let project_id = &task.project_id;
        let repos = match self.api.get_project_repos(project_id).await {
            Ok(repos) => repos,
            Err(e) => {
                tracing::error!("Failed to get project repos: {}", e);
                return Ok(());
            }
        };

        let Some(repo) = repos.first() else {
            tracing::warn!("Project has no repos configured");
            return Ok(());
        };

        let repo_id = repo.repo_id.clone();

        // Get or create attempt
        let attempt_id = if let Some(id) = &task.parent_workspace_id {
            id.clone()
        } else {
            // Auto-create an attempt for this task
            tracing::info!("Creating attempt for task to bind PR");
            let request = CreateTaskAttemptRequest {
                task_id: task.id.clone(),
                repos: vec![CreateTaskAttemptRepoRequest {
                    repo_id: repo_id.clone(),
                    target_branch: "main".to_string(), // Default target branch
                }],
            };
            match self.api.create_task_attempt(request).await {
                Ok(attempt) => attempt.id,
                Err(e) => {
                    tracing::error!("Failed to create attempt: {}", e);
                    return Ok(());
                }
            }
        };

        // Suspend terminal for fzf
        terminal.suspend()?;

        // List PRs using gh CLI
        let prs = match list_prs(20, None) {
            Ok(prs) => prs,
            Err(e) => {
                terminal.resume()?;
                tracing::error!("Failed to list PRs: {}", e);
                return Ok(());
            }
        };

        if prs.is_empty() {
            terminal.resume()?;
            tracing::warn!("No PRs found in repository");
            return Ok(());
        }

        // Select PR with fzf
        let selected_pr_number = match select_pr_with_fzf(&prs) {
            Ok(Some(num)) => num,
            Ok(None) => {
                terminal.resume()?;
                tracing::info!("PR selection cancelled");
                return Ok(());
            }
            Err(e) => {
                terminal.resume()?;
                tracing::error!("Failed to select PR: {}", e);
                return Ok(());
            }
        };

        // Resume terminal
        terminal.resume()?;

        // Bind the PR via API
        match self
            .api
            .bind_pr(&attempt_id, &repo_id, selected_pr_number)
            .await
        {
            Ok(response) => {
                if response.pr_attached {
                    tracing::info!(
                        "Bound PR #{} to task",
                        response.pr_number.unwrap_or(selected_pr_number)
                    );
                    // Refresh to show updated PR status
                    self.refresh().await?;
                } else {
                    tracing::warn!("Failed to bind PR");
                }
            }
            Err(e) => {
                tracing::error!("Failed to bind PR: {}", e);
            }
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

    async fn handle_sync_linear(&mut self) -> Result<()> {
        if !self.state.linear_api_key_available {
            tracing::warn!("Linear API key not available");
            return Ok(());
        }

        let Some(project_id) = self.state.selected_project_id.clone() else {
            tracing::warn!("No project selected for Linear sync");
            return Ok(());
        };

        tracing::info!("Syncing Linear backlog for project {}", project_id);

        match self.api.sync_linear_backlog(&project_id).await {
            Ok(response) => {
                tracing::info!(
                    "Linear sync complete: {} synced, {} created, {} updated",
                    response.synced_count,
                    response.created_count,
                    response.updated_count
                );
                // Refresh tasks to show newly synced items
                self.refresh().await?;
            }
            Err(e) => {
                tracing::error!("Failed to sync Linear backlog: {}", e);
            }
        }

        Ok(())
    }
}

/// Convert task title to a branch name slug
fn task_title_to_branch(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
