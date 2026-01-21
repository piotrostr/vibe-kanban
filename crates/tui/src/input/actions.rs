#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Navigation
    Up,
    Down,
    Left,
    Right,
    NextRow,
    PrevRow,

    // Selection
    Select,
    Back,
    Quit,

    // Task operations
    CreateTask,
    EditTask,
    DeleteTask,
    OpenTask,

    // Worktree operations
    ShowWorktrees,
    CreateWorktree,
    SwitchWorktree,

    // Session operations
    ShowSessions,
    LaunchSession,
    LaunchSessionPlan,
    AttachSession,
    KillSession,
    ViewPR,
    BindPR,

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

    // Linear integration
    SyncLinear,

    // Logs
    ShowLogs,
}
