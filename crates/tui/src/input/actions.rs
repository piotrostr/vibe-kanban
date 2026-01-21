#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Navigation
    Up,
    Down,
    Left,
    Right,

    // Selection
    Select,
    Back,
    Quit,

    // Task operations
    CreateTask,
    EditTask,
    DeleteTask,

    // Worktree operations
    ShowWorktrees,
    CreateWorktree,
    SwitchWorktree,

    // Session operations
    ShowSessions,
    LaunchSession,
    AttachSession,
    KillSession,

    // Search (vim-style /)
    StartSearch,
    SearchType(char),
    SearchBackspace,
    SearchConfirm,
    SearchCancel,
    ClearSearch,

    // Help
    ShowHelp,

    // Refresh
    Refresh,
}
