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

    // Attempt operations (legacy - simplified)
    StartAttempt,
    StopAttempt,
    OpenAttemptChat,

    // Chat input
    FocusInput,
    SendMessage,
    TypeChar(char),
    Backspace,

    // Worktree operations
    ShowWorktrees,
    CreateWorktree,
    SwitchWorktree,

    // Session operations
    ShowSessions,
    LaunchSession,
    AttachSession,
    KillSession,

    // Search
    FocusSearch,
    ClearSearch,

    // Help
    ShowHelp,

    // Refresh
    Refresh,
}
