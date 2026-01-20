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

    // Help
    ShowHelp,

    // Refresh
    Refresh,
}
