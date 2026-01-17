use std::path::{Path, PathBuf};

use anyhow;
use axum::{
    Extension, Json, Router,
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json as ResponseJson},
    routing::{delete, get, post, put},
};
use db::models::{
    execution_process::{
        CreateExecutionProcess, ExecutionProcess, ExecutionProcessRunReason,
        ExecutionProcessStatus,
    },
    execution_process_logs::ExecutionProcessLogs,
    image::TaskImage,
    project::{Project, ProjectError},
    project_repo::ProjectRepo,
    repo::Repo,
    session::{CreateSession, Session},
    task::{CreateTask, Task, TaskStatus, TaskWithAttemptStatus, UpdateTask},
    workspace::{CreateWorkspace, Workspace},
    workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo},
};
use deployment::Deployment;
use executors::{
    actions::{coding_agent_initial::CodingAgentInitialRequest, ExecutorAction, ExecutorActionType},
    executors::BaseCodingAgent,
    logs::{ActionType, NormalizedEntry, NormalizedEntryType, ToolStatus},
    profile::ExecutorProfileId,
};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use services::services::{
    container::ContainerService,
    linear::{LinearClient, LinearIssueWithState, linear_state_type_to_task_status},
    share::ShareError,
    workspace_manager::WorkspaceManager,
};
use sqlx::Error as SqlxError;
use ts_rs::TS;
use utils::{api::oauth::LoginStatus, log_msg::LogMsg, response::ApiResponse};
use uuid::Uuid;

use crate::claude_session::{
    self, ImportFromClaudeSessionRequest, ImportFromClaudeSessionResponse,
    ImportWithHistoryRequest, ImportWithHistoryResponse, ListClaudeSessionsResponse,
    PreviewClaudeSessionRequest, PreviewClaudeSessionResponse, get_session_cwd,
};

use crate::{
    DeploymentImpl, error::ApiError, middleware::load_task_middleware,
    routes::task_attempts::WorkspaceRepoInput,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskQuery {
    pub project_id: Uuid,
}

pub async fn get_tasks(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskWithAttemptStatus>>>, ApiError> {
    let tasks =
        Task::find_by_project_id_with_attempt_status(&deployment.db().pool, query.project_id)
            .await?;

    Ok(ResponseJson(ApiResponse::success(tasks)))
}

pub async fn stream_tasks_ws(
    ws: WebSocketUpgrade,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_tasks_ws(socket, deployment, query.project_id).await {
            tracing::warn!("tasks WS closed: {}", e);
        }
    })
}

async fn handle_tasks_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    project_id: Uuid,
) -> anyhow::Result<()> {
    // Get the raw stream and convert LogMsg to WebSocket messages
    let mut stream = deployment
        .events()
        .stream_tasks_raw(project_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Drain (and ignore) any client->server messages so pings/pongs work
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // Forward server messages
    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break; // client disconnected
                }
            }
            Err(e) => {
                tracing::error!("stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

/// WebSocket endpoint for streaming all tasks across all projects.
/// Used for the unified "Show All Projects" view.
pub async fn stream_all_tasks_ws(
    ws: WebSocketUpgrade,
    State(deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_all_tasks_ws(socket, deployment).await {
            tracing::warn!("all tasks WS closed: {}", e);
        }
    })
}

async fn handle_all_tasks_ws(socket: WebSocket, deployment: DeploymentImpl) -> anyhow::Result<()> {
    // Get the raw stream and convert LogMsg to WebSocket messages
    let mut stream = deployment
        .events()
        .stream_all_tasks_raw()
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Drain (and ignore) any client->server messages so pings/pongs work
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // Forward server messages
    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break; // client disconnected
                }
            }
            Err(e) => {
                tracing::error!("stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

pub async fn get_task(
    Extension(task): Extension<Task>,
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(task)))
}

pub async fn create_task(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let id = Uuid::new_v4();

    tracing::debug!(
        "Creating task '{}' in project {}",
        payload.title,
        payload.project_id
    );

    let task = Task::create(&deployment.db().pool, &payload, id).await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::associate_many_dedup(&deployment.db().pool, task.id, image_ids).await?;
    }

    Ok(ResponseJson(ApiResponse::success(task)))
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAndStartTaskRequest {
    pub task: CreateTask,
    pub executor_profile_id: ExecutorProfileId,
    pub repos: Vec<WorkspaceRepoInput>,
}

pub async fn create_task_and_start(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAndStartTaskRequest>,
) -> Result<ResponseJson<ApiResponse<TaskWithAttemptStatus>>, ApiError> {
    if payload.repos.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one repository is required".to_string(),
        ));
    }

    let pool = &deployment.db().pool;

    let task_id = Uuid::new_v4();
    let task = Task::create(pool, &payload.task, task_id).await?;

    if let Some(image_ids) = &payload.task.image_ids {
        TaskImage::associate_many_dedup(pool, task.id, image_ids).await?;
    }

    let project = Project::find_by_id(pool, task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    let attempt_id = Uuid::new_v4();
    let git_branch_name = deployment
        .container()
        .git_branch_from_workspace(&attempt_id, &task.title)
        .await;

    let agent_working_dir = project
        .default_agent_working_dir
        .as_ref()
        .filter(|dir: &&String| !dir.is_empty())
        .cloned();

    let workspace = Workspace::create(
        pool,
        &CreateWorkspace {
            branch: git_branch_name,
            agent_working_dir,
        },
        attempt_id,
        task.id,
    )
    .await?;

    let workspace_repos: Vec<CreateWorkspaceRepo> = payload
        .repos
        .iter()
        .map(|r| CreateWorkspaceRepo {
            repo_id: r.repo_id,
            target_branch: r.target_branch.clone(),
        })
        .collect();
    WorkspaceRepo::create_many(&deployment.db().pool, workspace.id, &workspace_repos).await?;

    let is_attempt_running = deployment
        .container()
        .start_workspace(&workspace, payload.executor_profile_id.clone())
        .await
        .inspect_err(|err| tracing::error!("Failed to start task attempt: {}", err))
        .is_ok();
    let task = Task::find_by_id(pool, task.id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    tracing::info!("Started attempt for task {}", task.id);
    Ok(ResponseJson(ApiResponse::success(TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: is_attempt_running,
        last_attempt_failed: false,
        executor: payload.executor_profile_id.executor.to_string(),
        pr_url: None,
        pr_status: None,
        pr_is_draft: None,
        pr_review_decision: None,
        pr_checks_status: None,
        pr_has_conflicts: None,
    })))
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportTaskFromPrRequest {
    pub project_id: Uuid,
    pub repo_id: Uuid,
    pub pr_number: i64,
    pub executor_profile_id: ExecutorProfileId,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type", rename_all = "snake_case")]
pub enum ImportTaskFromPrError {
    GithubCliNotInstalled,
    GithubCliNotLoggedIn,
    PrNotFoundOrNoAccess { pr_number: i64 },
}

pub async fn import_task_from_pr(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ImportTaskFromPrRequest>,
) -> Result<ResponseJson<ApiResponse<TaskWithAttemptStatus, ImportTaskFromPrError>>, ApiError> {
    use db::models::merge::{Merge, MergeStatus};
    use db::models::task::TaskStatus;
    use services::services::github::{GitHubService, GitHubServiceError};

    let pool = &deployment.db().pool;

    // Fetch repo info
    let repo = Repo::find_by_id(pool, payload.repo_id)
        .await?
        .ok_or(ApiError::BadRequest("Repository not found".to_string()))?;

    let github_service = GitHubService::new()?;
    let repo_info = deployment.git().get_github_repo_info(&repo.path)?;

    // Fetch PR info for import (title, body, branch)
    let pr_import_info = match github_service
        .view_pr_for_import(&repo_info, payload.pr_number)
        .await
    {
        Ok(info) => info,
        Err(e) => {
            tracing::error!(
                "Failed to fetch PR #{} for import: {}",
                payload.pr_number,
                e
            );
            return match &e {
                GitHubServiceError::GhCliNotInstalled(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::GithubCliNotInstalled),
                )),
                GitHubServiceError::AuthFailed(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::GithubCliNotLoggedIn),
                )),
                GitHubServiceError::RepoNotFoundOrNoAccess(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::PrNotFoundOrNoAccess {
                        pr_number: payload.pr_number,
                    }),
                )),
                _ => Err(ApiError::GitHubService(e)),
            };
        }
    };

    // Also fetch full PR status for binding
    let pr_status_info = match github_service
        .update_pr_status(&repo_info, payload.pr_number)
        .await
    {
        Ok(info) => info,
        Err(e) => {
            tracing::error!(
                "Failed to fetch PR #{} status: {}",
                payload.pr_number,
                e
            );
            return match &e {
                GitHubServiceError::GhCliNotInstalled(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::GithubCliNotInstalled),
                )),
                GitHubServiceError::AuthFailed(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::GithubCliNotLoggedIn),
                )),
                GitHubServiceError::RepoNotFoundOrNoAccess(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ImportTaskFromPrError::PrNotFoundOrNoAccess {
                        pr_number: payload.pr_number,
                    }),
                )),
                _ => Err(ApiError::GitHubService(e)),
            };
        }
    };

    // Create task from PR info
    let task_id = Uuid::new_v4();
    let description = if pr_import_info.body.is_empty() {
        None
    } else {
        Some(pr_import_info.body.clone())
    };
    let task = Task::create(
        pool,
        &CreateTask {
            project_id: payload.project_id,
            title: pr_import_info.title.clone(),
            description,
            status: None,
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            linear_issue_id: None,
            linear_url: None,
        },
        task_id,
    )
    .await?;

    let project = Project::find_by_id(pool, task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    // Create workspace using PR's branch name
    let attempt_id = Uuid::new_v4();
    let agent_working_dir = project
        .default_agent_working_dir
        .as_ref()
        .filter(|dir: &&String| !dir.is_empty())
        .cloned();

    let workspace = Workspace::create(
        pool,
        &CreateWorkspace {
            branch: pr_import_info.head_ref_name.clone(),
            agent_working_dir,
        },
        attempt_id,
        task.id,
    )
    .await?;

    // Create workspace repo with main as target branch
    let workspace_repos = vec![CreateWorkspaceRepo {
        repo_id: payload.repo_id,
        target_branch: "main".to_string(),
    }];
    WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;

    // Bind PR to workspace
    let mut tx = pool.begin().await?;
    let merge = Merge::create_pr_tx(
        &mut *tx,
        workspace.id,
        payload.repo_id,
        "main",
        pr_status_info.number,
        &pr_status_info.url,
    )
    .await?;

    // Update merge status if not open
    if !matches!(pr_status_info.status, MergeStatus::Open) {
        Merge::update_status_tx(
            &mut *tx,
            merge.id,
            pr_status_info.status.clone(),
            pr_status_info.merge_commit_sha.clone(),
        )
        .await?;
    }

    // If PR is merged, mark task as done
    if matches!(pr_status_info.status, MergeStatus::Merged) {
        Task::update_status(&mut *tx, task.id, TaskStatus::Done).await?;
    }

    tx.commit().await?;

    // Start workspace
    let is_attempt_running = deployment
        .container()
        .start_workspace(&workspace, payload.executor_profile_id.clone())
        .await
        .inspect_err(|err| tracing::error!("Failed to start task attempt: {}", err))
        .is_ok();

    let task = Task::find_by_id(pool, task.id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    tracing::info!(
        "Imported task {} from PR #{} ({})",
        task.id,
        payload.pr_number,
        pr_import_info.title
    );

    Ok(ResponseJson(ApiResponse::success(TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: is_attempt_running,
        last_attempt_failed: false,
        executor: payload.executor_profile_id.executor.to_string(),
        pr_url: Some(pr_status_info.url),
        pr_status: Some(pr_status_info.status),
        pr_is_draft: Some(pr_status_info.is_draft),
        pr_review_decision: Some(pr_status_info.review_decision),
        pr_checks_status: Some(pr_status_info.checks_status),
        pr_has_conflicts: Some(pr_status_info.has_conflicts),
    })))
}

pub async fn update_task(
    Extension(existing_task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,

    Json(payload): Json<UpdateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    ensure_shared_task_auth(&existing_task, &deployment).await?;

    // Use existing values if not provided in update
    let title = payload.title.unwrap_or(existing_task.title.clone());
    let description = match payload.description {
        Some(s) if s.trim().is_empty() => None, // Empty string = clear description
        Some(s) => Some(s),                     // Non-empty string = update description
        None => existing_task.description.clone(), // Field omitted = keep existing
    };
    let new_status = payload
        .status
        .clone()
        .unwrap_or(existing_task.status.clone());
    let parent_workspace_id = payload
        .parent_workspace_id
        .or(existing_task.parent_workspace_id);

    let task = Task::update(
        &deployment.db().pool,
        existing_task.id,
        existing_task.project_id,
        title,
        description,
        new_status.clone(),
        parent_workspace_id,
    )
    .await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::delete_by_task_id(&deployment.db().pool, task.id).await?;
        TaskImage::associate_many_dedup(&deployment.db().pool, task.id, image_ids).await?;
    }

    // If task has been shared, broadcast update
    if task.shared_task_id.is_some() {
        let Ok(publisher) = deployment.share_publisher() else {
            return Err(ShareError::MissingConfig("share publisher unavailable").into());
        };
        publisher.update_shared_task(&task).await?;
    }

    // If task originated from Linear, status changed, and user confirmed sync
    if payload.sync_to_linear
        && task.linear_issue_id.is_some()
        && payload.status.is_some()
        && existing_task.status != new_status
    {
        if let Some(linear_issue_id) = &task.linear_issue_id {
            // Get project to access Linear API key
            if let Ok(Some(project)) =
                Project::find_by_id(&deployment.db().pool, task.project_id).await
            {
                if let Some(api_key) = project.linear_api_key {
                    let client = LinearClient::new(api_key);
                    if let Err(e) = client
                        .sync_task_status_to_linear(linear_issue_id, &new_status)
                        .await
                    {
                        // Log warning but don't fail the local update
                        tracing::warn!("Failed to sync task {} status to Linear: {}", task.id, e);
                    } else {
                        tracing::info!(
                            "Synced task {} status to Linear: {:?}",
                            task.id,
                            new_status
                        );
                    }
                }
            }
        }
    }

    Ok(ResponseJson(ApiResponse::success(task)))
}

async fn ensure_shared_task_auth(
    existing_task: &Task,
    deployment: &local_deployment::LocalDeployment,
) -> Result<(), ApiError> {
    if existing_task.shared_task_id.is_some() {
        match deployment.get_login_status().await {
            LoginStatus::LoggedIn { .. } => return Ok(()),
            LoginStatus::LoggedOut => {
                return Err(ShareError::MissingAuth.into());
            }
        }
    }
    Ok(())
}

pub async fn delete_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    ensure_shared_task_auth(&task, &deployment).await?;

    // Validate no running execution processes
    if deployment
        .container()
        .has_running_processes(task.id)
        .await?
    {
        return Err(ApiError::Conflict("Task has running execution processes. Please wait for them to complete or stop them first.".to_string()));
    }

    let pool = &deployment.db().pool;

    // Gather task attempts data needed for background cleanup
    let attempts = Workspace::fetch_all(pool, Some(task.id))
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task attempts for task {}: {}", task.id, e);
            ApiError::Workspace(e)
        })?;

    let repositories = WorkspaceRepo::find_unique_repos_for_task(pool, task.id).await?;

    // Collect workspace directories that need cleanup
    let workspace_dirs: Vec<PathBuf> = attempts
        .iter()
        .filter_map(|attempt| attempt.container_ref.as_ref().map(PathBuf::from))
        .collect();

    if let Some(shared_task_id) = task.shared_task_id {
        let Ok(publisher) = deployment.share_publisher() else {
            return Err(ShareError::MissingConfig("share publisher unavailable").into());
        };
        publisher.delete_shared_task(shared_task_id).await?;
    }

    // Use a transaction to ensure atomicity: either all operations succeed or all are rolled back
    let mut tx = pool.begin().await?;

    // Nullify parent_workspace_id for all child tasks before deletion
    // This breaks parent-child relationships to avoid foreign key constraint violations
    let mut total_children_affected = 0u64;
    for attempt in &attempts {
        let children_affected =
            Task::nullify_children_by_workspace_id(&mut *tx, attempt.id).await?;
        total_children_affected += children_affected;
    }

    // Delete task from database (FK CASCADE will handle task_attempts)
    let rows_affected = Task::delete(&mut *tx, task.id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(SqlxError::RowNotFound));
    }

    // Commit the transaction - if this fails, all changes are rolled back
    tx.commit().await?;

    if total_children_affected > 0 {
        tracing::info!(
            "Nullified {} child task references before deleting task {}",
            total_children_affected,
            task.id
        );
    }

    let task_id = task.id;
    let pool = pool.clone();
    tokio::spawn(async move {
        tracing::info!(
            "Starting background cleanup for task {} ({} workspaces, {} repos)",
            task_id,
            workspace_dirs.len(),
            repositories.len()
        );

        for workspace_dir in &workspace_dirs {
            if let Err(e) = WorkspaceManager::cleanup_workspace(workspace_dir, &repositories).await
            {
                tracing::error!(
                    "Background workspace cleanup failed for task {} at {}: {}",
                    task_id,
                    workspace_dir.display(),
                    e
                );
            }
        }

        match Repo::delete_orphaned(&pool).await {
            Ok(count) if count > 0 => {
                tracing::info!("Deleted {} orphaned repo records", count);
            }
            Err(e) => {
                tracing::error!("Failed to delete orphaned repos: {}", e);
            }
            _ => {}
        }

        tracing::info!("Background cleanup completed for task {}", task_id);
    });

    // Return 202 Accepted to indicate deletion was scheduled
    Ok((StatusCode::ACCEPTED, ResponseJson(ApiResponse::success(()))))
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ShareTaskResponse {
    pub shared_task_id: Uuid,
}

pub async fn share_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ShareTaskResponse>>, ApiError> {
    let Ok(publisher) = deployment.share_publisher() else {
        return Err(ShareError::MissingConfig("share publisher unavailable").into());
    };
    let profile = deployment
        .auth_context()
        .cached_profile()
        .await
        .ok_or(ShareError::MissingAuth)?;
    let shared_task_id = publisher.share_task(task.id, profile.user_id).await?;

    Ok(ResponseJson(ApiResponse::success(ShareTaskResponse {
        shared_task_id,
    })))
}

/// Response type for Linear issue state fetch
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct LinearIssueStateResponse {
    pub issue: LinearIssueWithState,
    pub mapped_status: db::models::task::TaskStatus,
}

/// Fetch the current state of a Linear issue linked to a task
pub async fn get_linear_issue_state(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<LinearIssueStateResponse>>, ApiError> {
    let linear_issue_id = task
        .linear_issue_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Task is not linked to a Linear issue".to_string()))?;

    let project = Project::find_by_id(&deployment.db().pool, task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    let api_key = project.linear_api_key.ok_or_else(|| {
        ApiError::BadRequest("Project does not have a Linear API key configured".to_string())
    })?;

    let client = LinearClient::new(api_key);
    let issue = client
        .fetch_issue(linear_issue_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to fetch Linear issue: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("Linear issue not found".to_string()))?;

    let mapped_status = linear_state_type_to_task_status(&issue.state.state_type);

    Ok(ResponseJson(ApiResponse::success(
        LinearIssueStateResponse {
            issue,
            mapped_status,
        },
    )))
}

/// Pull the latest state from Linear and update the local task
pub async fn pull_from_linear(
    Extension(existing_task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let linear_issue_id = existing_task
        .linear_issue_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Task is not linked to a Linear issue".to_string()))?;

    let project = Project::find_by_id(&deployment.db().pool, existing_task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    let api_key = project.linear_api_key.ok_or_else(|| {
        ApiError::BadRequest("Project does not have a Linear API key configured".to_string())
    })?;

    let client = LinearClient::new(api_key);
    let issue = client
        .fetch_issue(linear_issue_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to fetch Linear issue: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("Linear issue not found".to_string()))?;

    let new_status = linear_state_type_to_task_status(&issue.state.state_type);

    // Update local task with Linear data
    let mut task = Task::update(
        &deployment.db().pool,
        existing_task.id,
        existing_task.project_id,
        issue.title,
        issue.description,
        new_status,
        existing_task.parent_workspace_id,
    )
    .await?;

    // Update labels from Linear
    let labels_json = if issue.labels.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&issue.labels).unwrap_or_default())
    };
    Task::update_linear_labels(&deployment.db().pool, task.id, labels_json.as_deref()).await?;
    task.linear_labels = labels_json;

    // If task has been shared, broadcast update
    if task.shared_task_id.is_some() {
        let Ok(publisher) = deployment.share_publisher() else {
            return Err(ShareError::MissingConfig("share publisher unavailable").into());
        };
        publisher.update_shared_task(&task).await?;
    }

    tracing::info!(
        "Pulled Linear issue {} to task {}: title='{}', status={:?}, labels_count={}",
        linear_issue_id,
        task.id,
        task.title,
        task.status,
        issue.labels.len()
    );

    Ok(ResponseJson(ApiResponse::success(task)))
}

/// Push local task state to Linear
pub async fn push_to_linear(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let linear_issue_id = task
        .linear_issue_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Task is not linked to a Linear issue".to_string()))?;

    let project = Project::find_by_id(&deployment.db().pool, task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    let api_key = project.linear_api_key.ok_or_else(|| {
        ApiError::BadRequest("Project does not have a Linear API key configured".to_string())
    })?;

    let client = LinearClient::new(api_key);
    client
        .sync_task_status_to_linear(linear_issue_id, &task.status)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to push to Linear: {}", e)))?;

    tracing::info!(
        "Pushed task {} status {:?} to Linear issue {}",
        task.id,
        task.status,
        linear_issue_id
    );

    Ok(ResponseJson(ApiResponse::success(())))
}

// Claude Session Import Routes

#[derive(Debug, Deserialize)]
pub struct ListClaudeSessionsQuery {
    pub project_path: Option<String>,
}

pub async fn list_claude_sessions(
    Query(query): Query<ListClaudeSessionsQuery>,
) -> Result<ResponseJson<ApiResponse<ListClaudeSessionsResponse>>, ApiError> {
    let sessions = claude_session::list_available_sessions(query.project_path.as_deref())
        .map_err(|e| ApiError::BadRequest(format!("Failed to list sessions: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(
        ListClaudeSessionsResponse { sessions },
    )))
}

pub async fn preview_claude_session(
    Json(payload): Json<PreviewClaudeSessionRequest>,
) -> Result<ResponseJson<ApiResponse<PreviewClaudeSessionResponse>>, ApiError> {
    let path = Path::new(&payload.session_path);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Session file not found: {}",
            payload.session_path
        )));
    }

    let items = claude_session::parse_session_file(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse session: {}", e)))?;

    let session_summary = claude_session::get_session_summary(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to get session summary: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(
        PreviewClaudeSessionResponse {
            items,
            session_summary,
        },
    )))
}

pub async fn import_from_claude_session(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
    Json(payload): Json<ImportFromClaudeSessionRequest>,
) -> Result<ResponseJson<ApiResponse<ImportFromClaudeSessionResponse>>, ApiError> {
    let path = Path::new(&payload.session_path);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Session file not found: {}",
            payload.session_path
        )));
    }

    let items = claude_session::parse_session_file(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse session: {}", e)))?;

    let default_status = payload
        .default_status
        .as_deref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "backlog" => Some(TaskStatus::Backlog),
            "todo" => Some(TaskStatus::Todo),
            "inprogress" => Some(TaskStatus::InProgress),
            _ => None,
        })
        .unwrap_or(TaskStatus::Backlog);

    let selected_ids: std::collections::HashSet<_> =
        payload.selected_item_ids.iter().cloned().collect();

    let items_to_import: Vec<_> = items
        .into_iter()
        .filter(|item| selected_ids.contains(&item.id))
        .collect();

    let mut imported_count = 0;
    let mut errors = Vec::new();

    for item in items_to_import {
        let task_id = Uuid::new_v4();
        let create_task = CreateTask {
            project_id: query.project_id,
            title: item.title,
            description: item.description,
            status: Some(default_status.clone()),
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            linear_issue_id: None,
            linear_url: None,
        };

        match Task::create(&deployment.db().pool, &create_task, task_id).await {
            Ok(_) => {
                imported_count += 1;
                tracing::info!("Imported task {} from Claude session", task_id);
            }
            Err(e) => {
                errors.push(format!("Failed to import task '{}': {}", item.id, e));
                tracing::error!("Failed to import task from Claude session: {}", e);
            }
        }
    }

    Ok(ResponseJson(ApiResponse::success(
        ImportFromClaudeSessionResponse {
            imported_count,
            errors,
        },
    )))
}

/// Import a Claude Code session with full conversation history.
/// Creates: Task -> Workspace -> Session -> ExecutionProcess -> ExecutionProcessLogs
pub async fn import_with_history(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
    Json(payload): Json<ImportWithHistoryRequest>,
) -> Result<ResponseJson<ApiResponse<ImportWithHistoryResponse>>, ApiError> {
    let path = Path::new(&payload.session_path);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Session file not found: {}",
            payload.session_path
        )));
    }

    let pool = &deployment.db().pool;

    // Get session slug for plan path and default title
    let session_slug = claude_session::get_session_slug(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to get session slug: {}", e)))?;

    // Get task title from the request or use slug/session_id
    let (title, description) = if let Some(custom_title) = &payload.task_title {
        (custom_title.clone(), None)
    } else {
        // Use slug as title (user can rename later), empty description
        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .to_string();
        let title = session_slug.clone().unwrap_or(session_id);
        (title, None)
    };

    // Parse status
    let status = payload
        .default_status
        .as_deref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "backlog" => Some(TaskStatus::Backlog),
            "todo" => Some(TaskStatus::Todo),
            "inprogress" => Some(TaskStatus::InProgress),
            "done" => Some(TaskStatus::Done),
            _ => None,
        })
        .unwrap_or(TaskStatus::Todo);

    // Extract raw session logs (1:1 parity with Claude Code JSONL)
    let log_lines = claude_session::extract_raw_session_logs(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to extract logs: {}", e)))?;

    // Get session info for branch name
    let session_info = claude_session::list_available_sessions(None)
        .map_err(|e| ApiError::BadRequest(format!("Failed to list sessions: {}", e)))?
        .into_iter()
        .find(|s| s.path == payload.session_path);

    let branch = session_info
        .as_ref()
        .and_then(|s| s.git_branch.clone())
        .unwrap_or_else(|| format!("imported-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")));

    let claude_session_id = session_info
        .as_ref()
        .map(|s| s.session_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 1. Create Task
    let task_id = Uuid::new_v4();
    let task = Task::create(
        pool,
        &CreateTask {
            project_id: query.project_id,
            title,
            description,
            status: Some(status),
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            linear_issue_id: None,
            linear_url: None,
        },
        task_id,
    )
    .await?;

    // 2. Create Workspace
    let workspace_id = Uuid::new_v4();
    let workspace = Workspace::create(
        pool,
        &CreateWorkspace {
            branch: branch.clone(),
            agent_working_dir: None,
        },
        workspace_id,
        task.id,
    )
    .await?;

    // 2b. Extract cwd from session and check if it's an existing worktree
    let session_cwd = get_session_cwd(path)
        .map_err(|e| ApiError::BadRequest(format!("Failed to get session cwd: {}", e)))?;

    // Check if the cwd is already a registered worktree
    // Worktrees have .git as a file (pointing to main repo), not a directory
    let is_existing_worktree = session_cwd.as_ref().map_or(false, |cwd| {
        let git_path = Path::new(cwd).join(".git");
        git_path.is_file()
    });

    if is_existing_worktree {
        // Case 1: Already a worktree - use it directly as container_ref
        if let Some(cwd) = &session_cwd {
            Workspace::update_container_ref(pool, workspace.id, cwd).await?;
            tracing::info!(
                "Imported session uses existing worktree: {} for workspace {}",
                cwd,
                workspace.id
            );
        }
        // Skip workspace repo creation - we're using existing worktree as-is
    } else {
        // Case 2: Not a worktree - add repos so the system creates one
        let project_repos = ProjectRepo::find_by_project_id(pool, query.project_id).await?;
        if !project_repos.is_empty() {
            let workspace_repos: Vec<CreateWorkspaceRepo> = project_repos
                .iter()
                .map(|pr| CreateWorkspaceRepo {
                    repo_id: pr.repo_id,
                    target_branch: String::new(), // Use default branch
                })
                .collect();
            WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;
        }
    }

    // 3. Create Session
    let session_id = Uuid::new_v4();
    let session = Session::create(
        pool,
        &CreateSession {
            executor: Some("CLAUDE_CODE".to_string()),
        },
        session_id,
        workspace.id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create session: {}", e)))?;

    // 4. Create ExecutionProcess (marked as Completed)
    let execution_process_id = Uuid::new_v4();
    let executor_action = ExecutorAction::new(
        ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
            prompt: format!("Imported from Claude Code session: {}", claude_session_id),
            executor_profile_id: ExecutorProfileId {
                executor: BaseCodingAgent::ClaudeCode,
                variant: None,
            },
            working_dir: None,
        }),
        None,
    );

    let execution_process = ExecutionProcess::create(
        pool,
        &CreateExecutionProcess {
            session_id: session.id,
            executor_action,
            run_reason: ExecutionProcessRunReason::ImportedSession,
        },
        execution_process_id,
        &[], // No repo states for imported sessions
    )
    .await?;

    // Mark the execution process as completed
    ExecutionProcess::update_completion(
        pool,
        execution_process.id,
        ExecutionProcessStatus::Completed,
        Some(0), // exit code 0 for success
    )
    .await?;

    // 5. Import log lines as a single batch (one row in the database)
    let log_lines_count = log_lines.len();
    let jsonl_lines: Vec<String> = log_lines
        .into_iter()
        .map(|line| {
            let log_msg = LogMsg::Stdout(line);
            serde_json::to_string(&log_msg)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::BadRequest(format!("Failed to serialize log: {}", e)))?;

    ExecutionProcessLogs::append_log_lines_batch(pool, execution_process.id, &jsonl_lines).await?;

    // 6. Import plan file if it exists
    if let Ok(Some(plan_path)) = claude_session::get_plan_path(path) {
        if let Ok(plan_content) = std::fs::read_to_string(&plan_path) {
            let plan_entry = NormalizedEntry {
                timestamp: None,
                entry_type: NormalizedEntryType::ToolUse {
                    tool_name: "ExitPlanMode".to_string(),
                    action_type: ActionType::PlanPresentation {
                        plan: plan_content.clone(),
                    },
                    status: ToolStatus::Success,
                },
                content: "Plan".to_string(),
                metadata: None,
            };
            // Wrap in LogMsg::Stdout containing the serialized NormalizedEntry
            let plan_json = serde_json::to_string(&plan_entry)
                .map_err(|e| ApiError::BadRequest(format!("Failed to serialize plan: {}", e)))?;
            let plan_log = LogMsg::Stdout(plan_json);
            let plan_log_str = serde_json::to_string(&plan_log)
                .map_err(|e| ApiError::BadRequest(format!("Failed to serialize plan log: {}", e)))?;
            ExecutionProcessLogs::append_log_lines_batch(pool, execution_process.id, &[plan_log_str])
                .await?;
            tracing::info!(
                "Imported plan from '{}' for task {}",
                plan_path.display(),
                task.id
            );
        }
    }

    tracing::info!(
        "Imported Claude session '{}' as task {} with {} log lines",
        claude_session_id,
        task.id,
        log_lines_count
    );

    Ok(ResponseJson(ApiResponse::success(ImportWithHistoryResponse {
        task_id: task.id.to_string(),
        workspace_id: workspace.id.to_string(),
        session_id: session.id.to_string(),
        execution_process_id: execution_process.id.to_string(),
        log_lines_imported: log_lines_count,
    })))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let task_actions_router = Router::new()
        .route("/", put(update_task))
        .route("/", delete(delete_task))
        .route("/share", post(share_task))
        .route("/linear", get(get_linear_issue_state))
        .route("/linear/pull", post(pull_from_linear))
        .route("/linear/push", post(push_to_linear));

    let task_id_router = Router::new()
        .route("/", get(get_task))
        .merge(task_actions_router)
        .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

    let inner = Router::new()
        .route("/", get(get_tasks).post(create_task))
        .route("/stream/ws", get(stream_tasks_ws))
        .route("/create-and-start", post(create_task_and_start))
        .route("/import-from-pr", post(import_task_from_pr))
        .route("/claude-sessions", get(list_claude_sessions))
        .route("/preview-claude-session", post(preview_claude_session))
        .route(
            "/import-from-claude-session",
            post(import_from_claude_session),
        )
        .route("/import-with-history", post(import_with_history))
        .nest("/{task_id}", task_id_router);

    // Top-level tasks routes (not scoped to a project)
    let all_tasks_router = Router::new()
        .route("/all/stream/ws", get(stream_all_tasks_ws));

    // mount under /projects/:project_id/tasks and /tasks
    Router::new()
        .nest("/tasks", inner.merge(all_tasks_router))
}
