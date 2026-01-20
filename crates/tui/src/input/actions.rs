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

    // Search
    FocusSearch,
    ClearSearch,

    // Help
    ShowHelp,

    // Refresh
    Refresh,
}
