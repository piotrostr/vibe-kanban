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

    // Attempt operations
    StartAttempt,
    StopAttempt,
    OpenAttemptChat,

    // Chat input
    FocusInput,
    SendMessage,
    TypeChar(char),
    Backspace,

    // Search
    FocusSearch,
    ClearSearch,

    // Help
    ShowHelp,

    // Refresh
    Refresh,
}
