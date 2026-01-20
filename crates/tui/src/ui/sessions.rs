use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::SessionsState;

pub fn render_sessions(frame: &mut Frame, area: Rect, state: &SessionsState) {
    if let Some(error) = &state.error {
        let error_msg = Paragraph::new(format!("Error: {}", error))
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Zellij Sessions ")
                    .border_style(Style::default().fg(Color::Red)),
            );
        frame.render_widget(error_msg, area);
        return;
    }

    if state.loading {
        let loading = Paragraph::new("Loading sessions...").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Zellij Sessions ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(loading, area);
        return;
    }

    if state.sessions.is_empty() {
        let empty = Paragraph::new("No active zellij sessions. Press 's' on a task to start one.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Zellij Sessions ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = i == state.selected_index;
            let is_vibe = session.name.starts_with("vibe-");

            let base_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_vibe {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let current_marker = if session.is_current {
                Span::styled(" (attached)", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            };

            let vibe_marker = if is_vibe {
                Span::styled("[vibe] ", Style::default().fg(Color::Magenta))
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                Span::raw(if is_selected { "> " } else { "  " }),
                vibe_marker,
                Span::styled(&session.name, base_style),
                current_marker,
            ]))
        })
        .collect();

    let vibe_count = state.vibe_sessions().len();
    let title = format!(
        " Zellij Sessions ({} total, {} vibe) ",
        state.sessions.len(),
        vibe_count
    );

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}
