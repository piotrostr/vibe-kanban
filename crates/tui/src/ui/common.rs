use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::AppState;

pub fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match &state.selected_project_id {
        Some(id) => {
            let project_name = state
                .projects
                .projects
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown");
            format!(" Vibe - {} ", project_name)
        }
        None => " Vibe ".to_string(),
    };

    let status = if state.backend_connected {
        Span::styled(" Connected ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Disconnected ", Style::default().fg(Color::Red))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(&title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        status,
    ]))
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

pub fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let hints = match state.view {
        crate::state::View::Projects => "j/k: navigate | Enter: select | q: quit | ?: help",
        crate::state::View::Kanban => {
            "h/j/k/l: navigate | Enter: details | c: create | d: delete | s: start | Esc: back"
        }
        crate::state::View::TaskDetail => "j/k: scroll | e: edit | s: start | Esc: back",
    };

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));

    frame.render_widget(footer, area);
}

pub fn render_help_modal(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  h/j/k/l or arrows  Move around"),
        Line::from("  Enter              Select / Open"),
        Line::from("  Esc / q            Back / Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tasks", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  c                  Create task"),
        Line::from("  e                  Edit task (nvim)"),
        Line::from("  d                  Delete task"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Attempts", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  s                  Start attempt"),
        Line::from("  S                  Stop attempt"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Other", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  /                  Search"),
        Line::from("  r                  Refresh"),
        Line::from("  ?                  This help"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press Esc to close",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    // Center the modal
    let modal_width = 50;
    let modal_height = help_text.len() as u16 + 2;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear the area behind the modal
    let clear = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(clear, modal_area);

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(help, modal_area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
