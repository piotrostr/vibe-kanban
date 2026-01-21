#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    NextRow,
    PrevRow,

    Select,
    Back,
    Quit,

    CreateTask,
    EditTask,
    DeleteTask,
    OpenTask,

    ShowWorktrees,
    CreateWorktree,
    SwitchWorktree,

    ShowSessions,
    LaunchSession,
    LaunchSessionPlan,
    AttachSession,
    KillSession,
    ViewPR,
    BindPR,

    StartSearch,
    SearchType(char),
    SearchBackspace,
    SearchDeleteWord,
    ClearSearch,

    // Command mode (vim-like ;f)
    StartCommand,
    CommandType(char),
    CommandBackspace,
    ExecuteCommand,
    CancelCommand,

    ShowHelp,
    Refresh,
    SyncLinear,
    ShowLogs,
}
