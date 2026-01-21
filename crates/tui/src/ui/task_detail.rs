use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::Task;

pub fn render_task_detail(frame: &mut Frame, area: Rect, task: &Task) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Status & metadata
            Constraint::Min(0),     // Description
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title(" Task ")
        .border_style(Style::default().fg(Color::Cyan));

    let title = Paragraph::new(task.title.clone())
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(title_block);

    frame.render_widget(title, chunks[0]);

    // Status & metadata row
    let status_color = match task.status {
        crate::state::TaskStatus::Backlog => Color::Gray,
        crate::state::TaskStatus::Todo => Color::Blue,
        crate::state::TaskStatus::Inprogress => Color::Yellow,
        crate::state::TaskStatus::Inreview => Color::Magenta,
        crate::state::TaskStatus::Done => Color::Green,
        crate::state::TaskStatus::Cancelled => Color::Red,
    };

    let mut metadata_spans = vec![
        Span::raw("Status: "),
        Span::styled(task.status.label(), Style::default().fg(status_color)),
    ];

    // Add attempt status
    if task.has_in_progress_attempt {
        metadata_spans.push(Span::raw(" | "));
        metadata_spans.push(Span::styled(
            "Running",
            Style::default().fg(Color::Yellow),
        ));
    } else if task.last_attempt_failed {
        metadata_spans.push(Span::raw(" | "));
        metadata_spans.push(Span::styled("Failed", Style::default().fg(Color::Red)));
    }

    // Add PR info
    if let Some(pr_url) = &task.pr_url {
        metadata_spans.push(Span::raw(" | PR: "));
        let pr_status_color = match task.pr_status.as_deref() {
            Some("merged") => Color::Magenta,
            Some("closed") => Color::Red,
            _ => Color::Green,
        };
        let pr_label = task
            .pr_status
            .as_deref()
            .unwrap_or("open")
            .to_uppercase();
        metadata_spans.push(Span::styled(pr_label, Style::default().fg(pr_status_color)));
    }

    // Add Linear info
    if let Some(linear_id) = &task.linear_issue_id {
        metadata_spans.push(Span::raw(" | Linear: "));
        metadata_spans.push(Span::styled(linear_id, Style::default().fg(Color::Blue)));
    }

    let metadata = Paragraph::new(Line::from(metadata_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Info ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(metadata, chunks[1]);

    // Description
    let description_text = task
        .description
        .as_deref()
        .unwrap_or("No description");

    let description = Paragraph::new(description_text)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Description ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(description, chunks[2]);
}

pub fn render_task_detail_with_actions(frame: &mut Frame, area: Rect, task: &Task) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Task detail
            Constraint::Length(3), // Actions bar
        ])
        .split(area);

    render_task_detail(frame, chunks[0], task);

    // Actions bar
    let actions = Paragraph::new(Line::from(vec![
        Span::styled("[g]", Style::default().fg(Color::Cyan)),
        Span::raw(" Gas it  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw(" Plan it  "),
        Span::styled("[v]", Style::default().fg(Color::Cyan)),
        Span::raw(" View PR  "),
        Span::styled("[e]", Style::default().fg(Color::Cyan)),
        Span::raw(" Edit  "),
        Span::styled("[d]", Style::default().fg(Color::Cyan)),
        Span::raw(" Delete  "),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Actions ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(actions, chunks[1]);
}
