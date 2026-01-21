use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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

/// Render logs as a centered overlay on top of everything
pub fn render_logs_overlay(frame: &mut Frame, area: Rect, logs: &LogsState) {
    // Create a centered overlay that takes up 80% of the screen
    let overlay_width = (area.width as f32 * 0.8) as u16;
    let overlay_height = (area.height as f32 * 0.8) as u16;

    let horizontal_margin = (area.width.saturating_sub(overlay_width)) / 2;
    let vertical_margin = (area.height.saturating_sub(overlay_height)) / 2;

    let overlay_area = Rect {
        x: area.x + horizontal_margin,
        y: area.y + vertical_margin,
        width: overlay_width,
        height: overlay_height,
    };

    // Clear the overlay area first
    frame.render_widget(Clear, overlay_area);

    let height = overlay_area.height.saturating_sub(2) as usize;

    let lines: Vec<Line> = logs
        .visible_lines(height)
        .map(|line| {
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

    frame.render_widget(paragraph, overlay_area);

    // Render help at bottom of overlay
    let help_text = " j/k: scroll | r: refresh | Shift+I/Esc: close ";
    let help_line = Line::from(vec![Span::styled(
        help_text,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )]);

    let help_area = Rect {
        x: overlay_area.x + 1,
        y: overlay_area.y + overlay_area.height - 1,
        width: help_text.len() as u16,
        height: 1,
    };

    frame.render_widget(Paragraph::new(help_line), help_area);
}
