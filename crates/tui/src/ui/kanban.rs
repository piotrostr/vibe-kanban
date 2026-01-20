use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::state::{TaskStatus, TasksState};

pub fn render_kanban_board(frame: &mut Frame, area: Rect, state: &TasksState) {
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
        let is_selected = state.selected_column == i;
        render_column(frame, columns[i], state, *status, is_selected);
    }
}

fn render_column(
    frame: &mut Frame,
    area: Rect,
    state: &TasksState,
    status: TaskStatus,
    is_selected: bool,
) {
    let tasks = state.tasks_in_column(status);
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
            let mut spans = vec![Span::raw(&task.title)];

            // Add status indicators
            if task.has_in_progress_attempt {
                spans.push(Span::styled(" [running]", Style::default().fg(Color::Yellow)));
            }
            if task.last_attempt_failed {
                spans.push(Span::styled(" [failed]", Style::default().fg(Color::Red)));
            }
            if task.pr_url.is_some() {
                let pr_color = match task.pr_status.as_deref() {
                    Some("merged") => Color::Magenta,
                    Some("closed") => Color::Red,
                    _ => Color::Green,
                };
                spans.push(Span::styled(" [PR]", Style::default().fg(pr_color)));
            }
            if task.linear_issue_id.is_some() {
                spans.push(Span::styled(" [Linear]", Style::default().fg(Color::Blue)));
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
        list_state.select(Some(state.selected_card_per_column[column_index]));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}
