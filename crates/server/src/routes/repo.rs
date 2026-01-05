use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::repo::Repo;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    git::GitBranch,
    github::{GitHubService, GitHubServiceError, PrListItem},
};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct RegisterRepoRequest {
    pub path: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct InitRepoRequest {
    pub parent_path: String,
    pub folder_name: String,
}

#[derive(Debug, Deserialize, TS)]
pub struct ListRecentPrsQuery {
    #[serde(default = "default_pr_limit")]
    pub limit: u32,
    pub search: Option<String>,
}

fn default_pr_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, TS)]
pub struct ListRecentPrsResponse {
    pub prs: Vec<PrListItem>,
}

#[derive(Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type", rename_all = "snake_case")]
pub enum ListRecentPrsError {
    GithubCliNotInstalled,
    GithubCliNotLoggedIn,
}

pub async fn register_repo(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<RegisterRepoRequest>,
) -> Result<ResponseJson<ApiResponse<Repo>>, ApiError> {
    let repo = deployment
        .repo()
        .register(
            &deployment.db().pool,
            &payload.path,
            payload.display_name.as_deref(),
        )
        .await?;

    Ok(ResponseJson(ApiResponse::success(repo)))
}

pub async fn init_repo(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<InitRepoRequest>,
) -> Result<ResponseJson<ApiResponse<Repo>>, ApiError> {
    let repo = deployment
        .repo()
        .init_repo(
            &deployment.db().pool,
            deployment.git(),
            &payload.parent_path,
            &payload.folder_name,
        )
        .await?;

    Ok(ResponseJson(ApiResponse::success(repo)))
}

pub async fn get_repo_branches(
    State(deployment): State<DeploymentImpl>,
    Path(repo_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<GitBranch>>>, ApiError> {
    let repo = deployment
        .repo()
        .get_by_id(&deployment.db().pool, repo_id)
        .await?;

    let branches = deployment.git().get_all_branches(&repo.path)?;
    Ok(ResponseJson(ApiResponse::success(branches)))
}

pub async fn list_recent_prs(
    State(deployment): State<DeploymentImpl>,
    Path(repo_id): Path<Uuid>,
    Query(query): Query<ListRecentPrsQuery>,
) -> Result<ResponseJson<ApiResponse<ListRecentPrsResponse, ListRecentPrsError>>, ApiError> {
    let repo = deployment
        .repo()
        .get_by_id(&deployment.db().pool, repo_id)
        .await?;

    let repo_info = deployment.git().get_github_repo_info(&repo.path)?;
    let github_service = GitHubService::new()?;

    match github_service
        .list_recent_prs(&repo_info, query.limit, query.search.as_deref())
        .await
    {
        Ok(prs) => Ok(ResponseJson(ApiResponse::success(ListRecentPrsResponse {
            prs,
        }))),
        Err(e) => {
            tracing::error!("Failed to list recent PRs for repo {}: {}", repo_id, e);
            match &e {
                GitHubServiceError::GhCliNotInstalled(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ListRecentPrsError::GithubCliNotInstalled),
                )),
                GitHubServiceError::AuthFailed(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(ListRecentPrsError::GithubCliNotLoggedIn),
                )),
                _ => Err(ApiError::GitHubService(e)),
            }
        }
    }
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/repos", post(register_repo))
        .route("/repos/init", post(init_repo))
        .route("/repos/{repo_id}/branches", get(get_repo_branches))
        .route("/repos/{repo_id}/prs", get(list_recent_prs))
}
