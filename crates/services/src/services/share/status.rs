use db::models::task::TaskStatus;
use remote::db::tasks::TaskStatus as RemoteTaskStatus;

pub(super) fn to_remote(status: &TaskStatus) -> RemoteTaskStatus {
    match status {
        // Backlog maps to Todo for remote sharing (backlog is local Linear-specific status)
        TaskStatus::Backlog => RemoteTaskStatus::Todo,
        TaskStatus::Todo => RemoteTaskStatus::Todo,
        TaskStatus::InProgress => RemoteTaskStatus::InProgress,
        TaskStatus::InReview => RemoteTaskStatus::InReview,
        TaskStatus::Done => RemoteTaskStatus::Done,
        TaskStatus::Cancelled => RemoteTaskStatus::Cancelled,
    }
}
