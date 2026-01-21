use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Action;
use crate::state::View;

pub fn key_to_action(
    key: KeyEvent,
    view: View,
    in_modal: bool,
    search_active: bool,
) -> Option<Action> {
    // Modal-specific bindings
    if in_modal {
        return match key.code {
            KeyCode::Esc => Some(Action::Back),
            KeyCode::Enter => Some(Action::Select),
            _ => None,
        };
    }

    // Search mode bindings - capture all input for search
    if search_active {
        return search_bindings(key);
    }

    // Global bindings
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => return Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Some(Action::Quit),
        (KeyCode::Char('?'), KeyModifiers::NONE) => return Some(Action::ShowHelp),
        (KeyCode::Char('/'), KeyModifiers::NONE) => return Some(Action::StartSearch),
        (KeyCode::Esc, _) => return Some(Action::Back),
        _ => {}
    }

    // View-specific bindings
    match view {
        View::Projects => project_list_bindings(key),
        View::Kanban => kanban_bindings(key),
        View::TaskDetail => task_detail_bindings(key),
        View::Worktrees => worktrees_bindings(key),
        View::Sessions => sessions_bindings(key),
    }
}

fn search_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::SearchCancel),
        KeyCode::Enter => Some(Action::SearchConfirm),
        KeyCode::Backspace => Some(Action::SearchBackspace),
        KeyCode::Char(c) => Some(Action::SearchType(c)),
        _ => None,
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

        // Launch Claude Code session
        KeyCode::Char('g') => Some(Action::LaunchSession),
        KeyCode::Char('p') => Some(Action::LaunchSessionPlan),
        KeyCode::Char('v') => Some(Action::ViewPR),

        // Worktrees and sessions views
        KeyCode::Char('w') => Some(Action::ShowWorktrees),
        KeyCode::Char('W') => Some(Action::CreateWorktree),
        KeyCode::Char('S') => Some(Action::ShowSessions),

        // Linear sync
        KeyCode::Char('L') => Some(Action::SyncLinear),

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
        KeyCode::Char('g') => Some(Action::LaunchSession),
        KeyCode::Char('p') => Some(Action::LaunchSessionPlan),
        KeyCode::Char('v') => Some(Action::ViewPR),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::LaunchSession),
        KeyCode::Char('w') => Some(Action::ShowWorktrees),
        KeyCode::Char('S') => Some(Action::ShowSessions),
        _ => None,
    }
}

fn worktrees_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::SwitchWorktree),
        KeyCode::Char('g') => Some(Action::LaunchSession),
        KeyCode::Char('p') => Some(Action::LaunchSessionPlan),
        KeyCode::Char('W') => Some(Action::CreateWorktree),
        KeyCode::Char('S') => Some(Action::ShowSessions),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}

fn sessions_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter | KeyCode::Char('a') => Some(Action::AttachSession),
        KeyCode::Char('K') => Some(Action::KillSession),
        KeyCode::Char('w') => Some(Action::ShowWorktrees),
        KeyCode::Char('r') => Some(Action::Refresh),
        _ => None,
    }
}
