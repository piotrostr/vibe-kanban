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
        (KeyCode::Char('I'), KeyModifiers::SHIFT) => return Some(Action::ShowLogs),
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
        View::Logs => logs_bindings(key),
        View::Search => search_bindings(key),
    }
}

fn search_bindings(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        // Navigation with Ctrl-j/k while typing
        (KeyCode::Char('j'), KeyModifiers::CONTROL) => Some(Action::Down),
        (KeyCode::Char('k'), KeyModifiers::CONTROL) => Some(Action::Up),
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => Some(Action::Down),
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => Some(Action::Up),
        (KeyCode::Down, _) => Some(Action::Down),
        (KeyCode::Up, _) => Some(Action::Up),
        // Ctrl-w to delete word (like shell)
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => Some(Action::SearchDeleteWord),
        // Ctrl-u to clear line
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Some(Action::ClearSearch),
        // Esc to close search
        (KeyCode::Esc, _) => Some(Action::Back),
        // Enter to select and go to task
        (KeyCode::Enter, _) => Some(Action::Select),
        // Backspace to delete char
        (KeyCode::Backspace, _) => Some(Action::SearchBackspace),
        // Any other char is typed into search
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            Some(Action::SearchType(c))
        }
        _ => None,
    }
}

fn logs_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Char('r') => Some(Action::Refresh),
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
    match (key.code, key.modifiers) {
        // Navigation within current row (status section)
        (KeyCode::Char('j') | KeyCode::Down, KeyModifiers::NONE) => Some(Action::Down),
        (KeyCode::Char('k') | KeyCode::Up, KeyModifiers::NONE) => Some(Action::Up),

        // Navigation between rows (status sections) - Shift+J/K
        (KeyCode::Char('J'), KeyModifiers::SHIFT) => Some(Action::NextRow),
        (KeyCode::Char('K'), KeyModifiers::SHIFT) => Some(Action::PrevRow),

        // Open task detail with 'l'
        (KeyCode::Char('l') | KeyCode::Right, KeyModifiers::NONE) => Some(Action::OpenTask),

        // Back with 'h'
        (KeyCode::Char('h') | KeyCode::Left, KeyModifiers::NONE) => Some(Action::Back),

        // Selection
        (KeyCode::Enter | KeyCode::Char(' '), KeyModifiers::NONE) => Some(Action::Select),

        // Task operations
        (KeyCode::Char('c'), KeyModifiers::NONE) => Some(Action::CreateTask),
        (KeyCode::Char('e'), KeyModifiers::NONE) => Some(Action::EditTask),
        (KeyCode::Char('d'), KeyModifiers::NONE) => Some(Action::DeleteTask),

        // Launch Claude Code session
        (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Action::LaunchSession),
        (KeyCode::Char('p'), KeyModifiers::NONE) => Some(Action::LaunchSessionPlan),
        (KeyCode::Char('v'), KeyModifiers::NONE) => Some(Action::ViewPR),
        (KeyCode::Char('b'), KeyModifiers::NONE) => Some(Action::BindPR),

        // Worktrees and sessions views
        (KeyCode::Char('w'), KeyModifiers::NONE) => Some(Action::ShowWorktrees),
        (KeyCode::Char('W'), KeyModifiers::SHIFT) => Some(Action::CreateWorktree),
        (KeyCode::Char('S'), KeyModifiers::SHIFT) => Some(Action::ShowSessions),

        // Linear sync
        (KeyCode::Char('L'), KeyModifiers::SHIFT) => Some(Action::SyncLinear),

        // Refresh
        (KeyCode::Char('r'), KeyModifiers::NONE) => Some(Action::Refresh),

        _ => None,
    }
}

fn task_detail_bindings(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Char('h') | KeyCode::Left => Some(Action::Back),
        KeyCode::Char('e') => Some(Action::EditTask),
        KeyCode::Char('g') => Some(Action::LaunchSession),
        KeyCode::Char('p') => Some(Action::LaunchSessionPlan),
        KeyCode::Char('v') => Some(Action::ViewPR),
        KeyCode::Char('b') => Some(Action::BindPR),
        KeyCode::Char('r') => Some(Action::Refresh),
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
