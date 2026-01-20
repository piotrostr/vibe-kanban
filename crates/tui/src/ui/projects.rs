use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::state::ProjectsState;

pub fn render_project_list(frame: &mut Frame, area: Rect, state: &ProjectsState) {
    let items: Vec<ListItem> = state
        .projects
        .iter()
        .map(|project| {
            let content = Line::from(vec![
                Span::styled(
                    &project.name,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                if project.remote_project_id.is_some() {
                    Span::styled(" (remote)", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("")
                },
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Projects ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}
