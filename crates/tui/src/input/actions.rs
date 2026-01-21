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

    ShowHelp,
    Refresh,
    SyncLinear,
    ShowLogs,
}
