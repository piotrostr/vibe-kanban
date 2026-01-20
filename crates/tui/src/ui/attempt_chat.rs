use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::{AttemptsState, Task, Workspace};

pub fn render_attempt_chat(
    frame: &mut Frame,
    area: Rect,
    task: &Task,
    attempts: &AttemptsState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Task title header
            Constraint::Length(8),  // Attempts list
            Constraint::Min(0),     // Chat/output area
            Constraint::Length(3),  // Input area
        ])
        .split(area);

    // Task title header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(&task.title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" - "),
        Span::styled(task.status.label(), Style::default().fg(Color::Yellow)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Task ")
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[0]);

    // Attempts list
    render_attempts_list(frame, chunks[1], attempts);

    // Chat/output area placeholder
    let output_text = if let Some(workspace) = attempts.selected_workspace() {
        format!(
            "Workspace: {}\nBranch: {}\n\n[Chat output will appear here]",
            &workspace.id[..8],
            workspace.branch
        )
    } else {
        "No attempt selected. Press [s] to start a new attempt.".to_string()
    };

    let output = Paragraph::new(output_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Output ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(output, chunks[2]);

    // Input area
    render_chat_input(frame, chunks[3], attempts);
}

fn render_attempts_list(frame: &mut Frame, area: Rect, attempts: &AttemptsState) {
    let items: Vec<ListItem> = attempts
        .workspaces
        .iter()
        .enumerate()
        .map(|(i, workspace)| {
            let is_selected = i == attempts.selected_workspace_index;
            let prefix = if is_selected { "> " } else { "  " };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_indicator = if workspace.setup_completed_at.is_some() {
                Span::styled(" [ready]", Style::default().fg(Color::Green))
            } else {
                Span::styled(" [setup]", Style::default().fg(Color::Yellow))
            };

            ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&workspace.branch, style),
                status_indicator,
                Span::styled(
                    format!(" ({})", &workspace.id[..8]),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Attempts ({}) ", attempts.workspaces.len()))
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn render_chat_input(frame: &mut Frame, area: Rect, attempts: &AttemptsState) {
    let input_style = if attempts.chat_input_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let placeholder = if attempts.chat_input.is_empty() {
        "Type a message... (press Enter to send, ! for shell command)"
    } else {
        ""
    };

    let input_text = if attempts.chat_input.is_empty() {
        placeholder.to_string()
    } else {
        attempts.chat_input.clone()
    };

    let cursor = if attempts.chat_input_active { "_" } else { "" };

    let input = Paragraph::new(format!("{}{}", input_text, cursor)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Message ")
            .border_style(input_style),
    );

    frame.render_widget(input, area);
}

pub fn render_attempt_actions(frame: &mut Frame, area: Rect) {
    let actions = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Focus input  "),
        Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
        Span::raw(" Select attempt  "),
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Send  "),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" Back  "),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(actions, area);
}
