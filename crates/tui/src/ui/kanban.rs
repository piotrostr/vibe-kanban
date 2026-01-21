use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::state::{SessionsState, TaskStatus, TasksState, WorktreesState};

pub fn render_kanban_board(
    frame: &mut Frame,
    area: Rect,
    tasks: &TasksState,
    worktrees: &WorktreesState,
    sessions: &SessionsState,
    spinner_char: char,
) {
    // Split into 4 columns (Backlog, In Progress, In Review, Done)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    for (i, status) in TaskStatus::VISIBLE.iter().enumerate() {
        let is_selected = tasks.selected_column == i;
        render_column(frame, columns[i], tasks, worktrees, sessions, *status, is_selected, spinner_char);
    }
}

fn render_column(
    frame: &mut Frame,
    area: Rect,
    tasks_state: &TasksState,
    worktrees: &WorktreesState,
    sessions: &SessionsState,
    status: TaskStatus,
    is_selected: bool,
    spinner_char: char,
) {
    let tasks = tasks_state.tasks_in_column(status);
    let count = tasks.len();
    let column_index = status.column_index();

    let title = format!(" {} ({}) ", status.label(), count);

    let border_color = if is_selected {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let items: Vec<ListItem> = tasks
        .iter()
        .map(|task| {
            // Row 1: Worktree/session status + activity indicator
            let mut row1_spans: Vec<Span> = vec![];

            // Activity indicator for in-progress tasks
            if task.has_in_progress_attempt {
                row1_spans.push(Span::styled(
                    format!("[{}] ", spinner_char),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ));
            } else if task.last_attempt_failed {
                row1_spans.push(Span::styled(
                    "[!] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ));
            }

            // Try to find matching worktree by task title (simplified matching)
            let task_slug = task.title.to_lowercase().replace(' ', "-");
            let matching_worktree = worktrees
                .worktrees
                .iter()
                .find(|w| w.branch.to_lowercase().contains(&task_slug) || task_slug.contains(&w.branch.to_lowercase()));

            if let Some(wt) = matching_worktree {
                // Show branch name
                let branch_display = if wt.branch.len() > 20 {
                    format!("{}...", &wt.branch[..17])
                } else {
                    wt.branch.clone()
                };
                row1_spans.push(Span::styled(
                    branch_display,
                    Style::default().fg(Color::Cyan),
                ));

                // Check for session
                if let Some(session) = sessions.session_for_branch(&wt.branch) {
                    if session.needs_attention {
                        row1_spans.push(Span::styled(" !", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                    } else {
                        row1_spans.push(Span::styled(" ", Style::default().fg(Color::Green)));
                    }
                }
            } else if !task.has_in_progress_attempt && !task.last_attempt_failed {
                row1_spans.push(Span::styled("no worktree", Style::default().fg(Color::DarkGray)));
            }

            // Row 2: Title + status indicators
            let mut row2_spans = vec![Span::raw(&task.title)];

            // PR status with more detail
            if task.pr_url.is_some() {
                let (pr_icon, pr_color) = match task.pr_status.as_deref() {
                    Some("merged") => ("", Color::Magenta),
                    Some("closed") => ("", Color::Red),
                    _ => {
                        // Check review/checks status for open PRs
                        match (task.pr_review_decision.as_deref(), task.pr_checks_status.as_deref()) {
                            (Some("APPROVED"), _) => ("", Color::Green),
                            (Some("CHANGES_REQUESTED"), _) => ("", Color::Yellow),
                            (_, Some("FAILURE")) => ("", Color::Red),
                            (_, Some("SUCCESS")) => ("", Color::Green),
                            _ => ("", Color::Cyan),
                        }
                    }
                };
                row2_spans.push(Span::styled(format!(" {}", pr_icon), Style::default().fg(pr_color)));

                if task.pr_has_conflicts == Some(true) {
                    row2_spans.push(Span::styled(" !", Style::default().fg(Color::Red)));
                }
            }

            if task.linear_issue_id.is_some() {
                row2_spans.push(Span::styled(" ", Style::default().fg(Color::Blue)));
            }

            ListItem::new(vec![Line::from(row1_spans), Line::from(row2_spans)])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if is_selected && !tasks.is_empty() {
        list_state.select(Some(tasks_state.selected_card_per_column[column_index]));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}
