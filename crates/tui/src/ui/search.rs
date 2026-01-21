use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::state::SearchState;

pub fn render_search(frame: &mut Frame, area: Rect, search: &SearchState) {
    // Split into search input (top) and results with preview (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(0),    // Results + preview
        ])
        .split(area);

    render_search_input(frame, chunks[0], search);
    render_results_and_preview(frame, chunks[1], search);
}

fn render_search_input(frame: &mut Frame, area: Rect, search: &SearchState) {
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(&search.query),
        Span::styled("_", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let input = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(input, area);
}

fn render_results_and_preview(frame: &mut Frame, area: Rect, search: &SearchState) {
    // Split horizontally: results list (left) and preview (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Results list
            Constraint::Percentage(60), // Preview
        ])
        .split(area);

    render_results_list(frame, chunks[0], search);
    render_preview(frame, chunks[1], search);
}

fn render_results_list(frame: &mut Frame, area: Rect, search: &SearchState) {
    let items: Vec<ListItem> = search
        .results
        .iter()
        .map(|result| ListItem::new(Line::from(result.title.clone())))
        .collect();

    let results_count = search.results.len();
    let title = if search.query.is_empty() {
        format!(" Tasks ({}) ", results_count)
    } else {
        format!(" Results ({}) ", results_count)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !search.results.is_empty() {
        list_state.select(Some(search.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_preview(frame: &mut Frame, area: Rect, search: &SearchState) {
    let content = if let Some(task) = search.selected_task() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Yellow)),
                Span::raw(&task.title),
            ]),
            Line::from(""),
        ];

        // Status
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::raw(task.status.label()),
        ]));

        // PR info if available
        if let Some(ref pr_url) = task.pr_url {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("PR: ", Style::default().fg(Color::Yellow)),
                Span::styled(pr_url, Style::default().fg(Color::Cyan)),
            ]));
            if let Some(ref status) = task.pr_status {
                lines.push(Line::from(vec![
                    Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(status),
                ]));
            }
        }

        // Linear info if available
        if let Some(ref linear_url) = task.linear_url {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Linear: ", Style::default().fg(Color::Yellow)),
                Span::styled(linear_url, Style::default().fg(Color::Blue)),
            ]));
        }

        // Description
        if let Some(ref desc) = task.description {
            if !desc.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Description:",
                    Style::default().fg(Color::Yellow),
                )));
                // Wrap description text
                for line in desc.lines().take(15) {
                    lines.push(Line::from(format!("  {}", line)));
                }
            }
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "No task selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let preview = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Preview ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(preview, area);
}
