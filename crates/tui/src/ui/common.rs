use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::AppState;

const LOGO: &str = r#"
 __   _(_) |__   ___
 \ \ / / | '_ \ / _ \
  \ V /| | |_) |  __/
   \_/ |_|_.__/ \___|"#;

pub fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    // If area is tall enough, render the ASCII logo
    if area.height >= 5 {
        render_header_with_logo(frame, area, state);
    } else {
        render_header_compact(frame, area, state);
    }
}

fn render_header_with_logo(frame: &mut Frame, area: Rect, state: &AppState) {
    let project_info = match &state.selected_project_id {
        Some(id) => {
            let project_name = state
                .projects
                .projects
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown");
            format!("Project: {}", project_name)
        }
        None => String::new(),
    };

    let status_text = if state.backend_connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status_color = if state.backend_connected {
        Color::Green
    } else {
        Color::Red
    };

    // Build lines: logo on left, status on right
    let logo_lines: Vec<&str> = LOGO.lines().skip(1).collect(); // Skip empty first line
    let mut lines: Vec<Line> = Vec::new();

    for (i, logo_line) in logo_lines.iter().enumerate() {
        let mut spans = vec![Span::styled(
            *logo_line,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )];

        // Add status info on the right side of the first few lines
        if i == 0 {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(status_text, Style::default().fg(status_color)));
        } else if i == 1 && !project_info.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                &project_info,
                Style::default().fg(Color::Yellow),
            ));
        }

        lines.push(Line::from(spans));
    }

    let header = Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_header_compact(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match &state.selected_project_id {
        Some(id) => {
            let project_name = state
                .projects
                .projects
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown");
            format!(" vibe - {} ", project_name)
        }
        None => " vibe ".to_string(),
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
    // Show search bar when active
    if state.search_active {
        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(&state.search_query),
            Span::styled("_", Style::default().fg(Color::Yellow)), // cursor
        ]);

        let footer = Paragraph::new(search_line)
            .style(Style::default())
            .block(Block::default().borders(Borders::TOP));

        frame.render_widget(footer, area);
        return;
    }

    // Show active search filter if present
    let search_indicator = if !state.search_query.is_empty() {
        format!(" [/{}] |", state.search_query)
    } else {
        String::new()
    };

    let hints = match state.view {
        crate::state::View::Projects => {
            format!(
                "{}j/k: navigate | Enter: select | /: search | q: quit | ?: help",
                search_indicator
            )
        }
        crate::state::View::Kanban => {
            format!(
                "{}h/j/k/l: nav | Enter: details | /: search | s: session | Esc: back",
                search_indicator
            )
        }
        crate::state::View::TaskDetail => {
            format!(
                "{}e: edit | s/Enter: session | /: search | Esc: back",
                search_indicator
            )
        }
        crate::state::View::Worktrees => {
            format!(
                "{}j/k: nav | Enter: switch | s: session | /: search | Esc: back",
                search_indicator
            )
        }
        crate::state::View::Sessions => {
            format!(
                "{}j/k: nav | Enter/a: attach | K: kill | /: search | Esc: back",
                search_indicator
            )
        }
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
            Span::styled("Worktrees", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  w                  Show worktrees"),
        Line::from("  W                  Create worktree"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Sessions", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  g                  Gas it (launch Claude)"),
        Line::from("  p                  Plan it (launch in plan mode)"),
        Line::from("  v                  View PR"),
        Line::from("  S                  Show sessions"),
        Line::from("  a / Enter          Attach to session"),
        Line::from("  K                  Kill session"),
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
