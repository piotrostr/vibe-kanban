use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::WorktreesState;

pub fn render_worktrees(frame: &mut Frame, area: Rect, state: &WorktreesState) {
    if let Some(error) = &state.error {
        let error_msg = Paragraph::new(format!("Error: {}", error))
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Worktrees ")
                    .border_style(Style::default().fg(Color::Red)),
            );
        frame.render_widget(error_msg, area);
        return;
    }

    if state.loading {
        let loading = Paragraph::new("Loading worktrees...").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Worktrees ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(loading, area);
        return;
    }

    if state.worktrees.is_empty() {
        let empty = Paragraph::new("No worktrees found. Press 'W' to create one.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Worktrees ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let is_selected = i == state.selected_index;

            // Build status indicators
            let current_marker = if wt.is_current { "*" } else { " " };
            let dirty_marker = if wt.is_dirty() { "!" } else { " " };
            let main_status = wt.status_symbol();

            // Ahead/behind counts
            let ahead_behind = wt
                .main
                .as_ref()
                .map(|m| {
                    if m.ahead > 0 && m.behind > 0 {
                        format!(" +{}-{}", m.ahead, m.behind)
                    } else if m.ahead > 0 {
                        format!(" +{}", m.ahead)
                    } else if m.behind > 0 {
                        format!(" -{}", m.behind)
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default();

            // Style based on selection and state
            let base_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if wt.is_current {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let status_style = if wt.is_dirty() {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let main_style = match wt.main_state.as_str() {
                "ahead" => Style::default().fg(Color::Green),
                "behind" => Style::default().fg(Color::Red),
                "diverged" => Style::default().fg(Color::Yellow),
                _ => Style::default().fg(Color::DarkGray),
            };

            ListItem::new(Line::from(vec![
                Span::raw(if is_selected { "> " } else { "  " }),
                Span::styled(current_marker, Style::default().fg(Color::Green)),
                Span::styled(dirty_marker, status_style),
                Span::styled(main_status, main_style),
                Span::raw(" "),
                Span::styled(&wt.branch, base_style),
                Span::styled(ahead_behind, main_style),
                Span::styled(
                    format!(" ({})", wt.short_commit()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Worktrees ({}) ", state.worktrees.len()))
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

pub fn render_worktree_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Legend: ", Style::default().fg(Color::DarkGray)),
        Span::styled("*", Style::default().fg(Color::Green)),
        Span::raw("=current "),
        Span::styled("!", Style::default().fg(Color::Yellow)),
        Span::raw("=dirty "),
        Span::styled("+", Style::default().fg(Color::Green)),
        Span::raw("=ahead "),
        Span::styled("-", Style::default().fg(Color::Red)),
        Span::raw("=behind"),
    ]))
    .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}
