use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::external::ClaudeActivityState;
use crate::state::SessionsState;

pub fn render_sessions(frame: &mut Frame, area: Rect, state: &SessionsState, spinner_char: char) {
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

            let base_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let status_marker = if session.is_current {
                Span::styled(" (attached)", Style::default().fg(Color::Yellow))
            } else if session.is_dead {
                Span::styled(" (dead)", Style::default().fg(Color::DarkGray))
            } else {
                // Use Claude activity state for more granular status
                match session.claude_activity {
                    ClaudeActivityState::Thinking => Span::styled(
                        format!(" [{}]", spinner_char),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    ClaudeActivityState::WaitingForUser => Span::styled(
                        " [!]",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    ClaudeActivityState::Idle => Span::styled(
                        " [-]",
                        Style::default().fg(Color::DarkGray),
                    ),
                    ClaudeActivityState::Unknown => {
                        // Fall back to legacy needs_attention check
                        if session.needs_attention {
                            Span::styled(
                                " [!]",
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled(" ", Style::default().fg(Color::Green))
                        }
                    }
                }
            };

            ListItem::new(Line::from(vec![
                Span::raw(if is_selected { "> " } else { "  " }),
                Span::styled(&session.name, base_style),
                status_marker,
            ]))
        })
        .collect();

    let waiting_count = state
        .sessions
        .iter()
        .filter(|s| {
            matches!(s.claude_activity, ClaudeActivityState::WaitingForUser)
                || (matches!(s.claude_activity, ClaudeActivityState::Unknown) && s.needs_attention)
        })
        .count();
    let title = if waiting_count > 0 {
        format!(
            " Zellij Sessions ({}) - {} waiting ",
            state.sessions.len(),
            waiting_count
        )
    } else {
        format!(" Zellij Sessions ({}) ", state.sessions.len())
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}
