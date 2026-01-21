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
    // Split into 4 horizontal rows (Backlog, In Progress, In Review, Done)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

    for (i, status) in TaskStatus::VISIBLE.iter().enumerate() {
        let is_selected = tasks.selected_column == i;
        render_row(frame, rows[i], tasks, worktrees, sessions, *status, is_selected, spinner_char);
    }
}

fn render_row(
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

    // For horizontal rows, show tasks in a single-line compact format
    let items: Vec<ListItem> = tasks
        .iter()
        .map(|task| {
            let mut spans: Vec<Span> = vec![];

            // Activity indicator
            if task.has_in_progress_attempt {
                spans.push(Span::styled(
                    format!("[{}] ", spinner_char),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ));
            } else if task.last_attempt_failed {
                spans.push(Span::styled(
                    "[!] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ));
            }

            // Title (truncate if too long)
            let max_title_len = 50;
            let title_display = if task.title.len() > max_title_len {
                format!("{}...", &task.title[..max_title_len - 3])
            } else {
                task.title.clone()
            };
            spans.push(Span::raw(title_display));

            // PR status
            if task.pr_url.is_some() {
                let (pr_icon, pr_color) = match task.pr_status.as_deref() {
                    Some("merged") => ("", Color::Magenta),
                    Some("closed") => ("", Color::Red),
                    _ => match (task.pr_review_decision.as_deref(), task.pr_checks_status.as_deref()) {
                        (Some("APPROVED"), _) => ("", Color::Green),
                        (Some("CHANGES_REQUESTED"), _) => ("", Color::Yellow),
                        (_, Some("FAILURE")) => ("", Color::Red),
                        (_, Some("SUCCESS")) => ("", Color::Green),
                        _ => ("", Color::Cyan),
                    },
                };
                spans.push(Span::styled(format!(" {}", pr_icon), Style::default().fg(pr_color)));
                if task.pr_has_conflicts == Some(true) {
                    spans.push(Span::styled(" !", Style::default().fg(Color::Red)));
                }
            }

            // Linear indicator
            if task.linear_issue_id.is_some() {
                spans.push(Span::styled(" ", Style::default().fg(Color::Blue)));
            }

            // Worktree/branch info
            let task_slug = task.title.to_lowercase().replace(' ', "-");
            let matching_worktree = worktrees
                .worktrees
                .iter()
                .find(|w| w.branch.to_lowercase().contains(&task_slug) || task_slug.contains(&w.branch.to_lowercase()));

            if let Some(wt) = matching_worktree {
                let branch_display = if wt.branch.len() > 15 {
                    format!(" ({}...)", &wt.branch[..12])
                } else {
                    format!(" ({})", wt.branch)
                };
                spans.push(Span::styled(branch_display, Style::default().fg(Color::DarkGray)));

                if let Some(session) = sessions.session_for_branch(&wt.branch) {
                    if session.needs_attention {
                        spans.push(Span::styled(" !", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                    } else {
                        spans.push(Span::styled(" ", Style::default().fg(Color::Green)));
                    }
                }
            }

            ListItem::new(Line::from(spans))
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
