use std::time::Duration;

use db::{
    DBService,
    models::{
        merge::{Merge, MergeStatus, PrMerge},
        task::{Task, TaskStatus},
        workspace::{Workspace, WorkspaceError},
    },
};
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::services::{
    github::{GitHubRepoInfo, GitHubService, GitHubServiceError},
    share::SharePublisher,
};

#[derive(Debug, Error)]
enum PrMonitorError {
    #[error(transparent)]
    GitHubServiceError(#[from] GitHubServiceError),
    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
}

/// Service to monitor GitHub PRs and update task status when they are merged
pub struct PrMonitorService {
    db: DBService,
    poll_interval: Duration,
    publisher: Option<SharePublisher>,
}

impl PrMonitorService {
    pub async fn spawn(
        db: DBService,
        publisher: Option<SharePublisher>,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            poll_interval: Duration::from_secs(60), // Check every minute
            publisher,
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting PR monitoring service with interval {:?}",
            self.poll_interval
        );

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_open_prs().await {
                error!("Error checking open PRs: {}", e);
            }
        }
    }

    /// Check all open PRs for updates with the provided GitHub token
    async fn check_all_open_prs(&self) -> Result<(), PrMonitorError> {
        let open_prs = Merge::get_open_prs(&self.db.pool).await?;

        if open_prs.is_empty() {
            debug!("No open PRs to check");
            return Ok(());
        }

        info!("Checking {} open PRs", open_prs.len());

        for pr_merge in open_prs {
            if let Err(e) = self.check_pr_status(&pr_merge).await {
                error!(
                    "Error checking PR #{} for workspace {}: {}",
                    pr_merge.pr_info.number, pr_merge.workspace_id, e
                );
            }
        }
        Ok(())
    }

    /// Check the status of a specific PR
    async fn check_pr_status(&self, pr_merge: &PrMerge) -> Result<(), PrMonitorError> {
        // GitHubService now uses gh CLI, no token needed
        let github_service = GitHubService::new()?;
        let repo_info = GitHubRepoInfo::from_remote_url(&pr_merge.pr_info.url)?;

        let pr_status = github_service
            .update_pr_status(&repo_info, pr_merge.pr_info.number)
            .await?;

        debug!(
            "PR #{} status: {:?}, review: {:?}, checks: {:?}",
            pr_merge.pr_info.number,
            pr_status.status,
            pr_status.review_decision,
            pr_status.checks_status
        );

        // Check if any field has changed
        let status_changed = pr_status.status != pr_merge.pr_info.status
            || pr_status.is_draft != pr_merge.pr_info.is_draft
            || pr_status.review_decision != pr_merge.pr_info.review_decision
            || pr_status.checks_status != pr_merge.pr_info.checks_status;

        if status_changed {
            // Update merge status with the latest information from GitHub
            Merge::update_status(&self.db.pool, pr_merge.id, &pr_status).await?;

            // If the PR was merged, update the task status to done
            if matches!(&pr_status.status, MergeStatus::Merged)
                && let Some(workspace) =
                    Workspace::find_by_id(&self.db.pool, pr_merge.workspace_id).await?
            {
                info!(
                    "PR #{} was merged, updating task {} to done",
                    pr_merge.pr_info.number, workspace.task_id
                );
                Task::update_status(&self.db.pool, workspace.task_id, TaskStatus::Done).await?;

                if let Some(publisher) = &self.publisher
                    && let Err(err) = publisher.update_shared_task_by_id(workspace.task_id).await
                {
                    tracing::warn!(
                        ?err,
                        "Failed to propagate shared task update for {}",
                        workspace.task_id
                    );
                }
            }
        }

        Ok(())
    }
}
