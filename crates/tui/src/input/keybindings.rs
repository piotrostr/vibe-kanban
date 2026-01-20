use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Action;
use crate::state::View;

pub fn key_to_action(key: KeyEvent, view: View, in_modal: bool, chat_input_active: bool) -> Option<Action> {
    // Modal-specific bindings
    if in_modal {
        return match key.code {
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Enter => Some(Action::Select),
            _ => None,
        };
    }

    // Chat input mode - capture all keys for typing
    if chat_input_active {
        return chat_input_bindings(key);
    }

    // Global bindings
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => return Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Some(Action::Quit),
        (KeyCode::Char('?'), KeyModifiers::NONE) => return Some(Action::ShowHelp),
        (KeyCode::Esc, _) => return Some(Action::Back),
        _ => {}
    }

    // View-specific bindings
    match view {
        View::Projects => project_list_bindings(key),
        View::Kanban => kanban_bindings(key),
        View::TaskDetail => task_detail_bindings(key),
        View::AttemptChat => attempt_chat_bindings(key),
    }
}

fn project_list_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Select),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

fn kanban_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Char('h') | KeyCode::Left => Some(Action::Left),
        KeyCode::Char('l') | KeyCode::Right => Some(Action::Right),

        // Selection
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Select),

        // Task operations
        KeyCode::Char('c') => Some(Action::CreateTask),
        KeyCode::Char('e') => Some(Action::EditTask),
        KeyCode::Char('d') => Some(Action::DeleteTask),

        // Attempt operations
        KeyCode::Char('s') => Some(Action::StartAttempt),
        KeyCode::Char('S') => Some(Action::StopAttempt),

        // Search
        KeyCode::Char('/') => Some(Action::FocusSearch),

        // Refresh
        KeyCode::Char('r') => Some(Action::Refresh),

        _ => None,
    }
}

fn task_detail_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Char('e') => Some(Action::EditTask),
        KeyCode::Char('s') => Some(Action::StartAttempt),
        KeyCode::Char('S') => Some(Action::StopAttempt),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::OpenAttemptChat),
        _ => None,
    }
}

fn attempt_chat_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Tab | KeyCode::Char('i') => Some(Action::FocusInput),
        KeyCode::Char('s') => Some(Action::StartAttempt),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

fn chat_input_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Enter => Some(Action::SendMessage),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char(c) => Some(Action::TypeChar(c)),
        _ => None,
    }
}
