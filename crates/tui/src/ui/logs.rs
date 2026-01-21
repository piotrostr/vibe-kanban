use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::LogsState;

pub fn render_logs(frame: &mut Frame, area: Rect, logs: &LogsState) {
    let height = area.height.saturating_sub(2) as usize; // Account for borders

    let lines: Vec<Line> = logs
        .visible_lines(height)
        .map(|line| {
            // Color based on log level
            let style = if line.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else if line.contains("WARN") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("INFO") {
                Style::default().fg(Color::Green)
            } else if line.contains("DEBUG") {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Line::from(Span::styled(line.clone(), style))
        })
        .collect();

    let title = format!(
        " Logs ({}) - {} ",
        logs.lines.len(),
        logs.log_path.display()
    );

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Render help at bottom
    let help_text = " j/k: scroll | r: refresh | Esc: back ";
    let help_line = Line::from(vec![Span::styled(
        help_text,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )]);

    let help_area = Rect {
        x: area.x + 1,
        y: area.y + area.height - 1,
        width: help_text.len() as u16,
        height: 1,
    };

    frame.render_widget(Paragraph::new(help_line), help_area);
}
