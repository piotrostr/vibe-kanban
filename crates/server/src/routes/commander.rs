use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::{
    commander_session::CommanderSession, execution_process::ExecutionProcess, project::Project,
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_project_middleware};

#[derive(Debug, Deserialize, TS)]
pub struct CreateFollowUpRequest {
    pub prompt: String,
    pub variant: Option<String>,
}

/// Get or create the commander session for a project
pub async fn get_or_create_commander(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<CommanderSession>>, ApiError> {
    let pool = &deployment.db().pool;

    // Find or create commander session
    let commander_session = CommanderSession::find_or_create(pool, project.id, None).await?;

    Ok(ResponseJson(ApiResponse::success(commander_session)))
}

/// Get a commander session by its ID
pub async fn get_commander(
    Extension(commander_session): Extension<CommanderSession>,
) -> Result<ResponseJson<ApiResponse<CommanderSession>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(commander_session)))
}

/// Get all execution processes for a commander session
pub async fn get_commander_processes(
    Extension(commander_session): Extension<CommanderSession>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ExecutionProcess>>>, ApiError> {
    let pool = &deployment.db().pool;

    let processes =
        ExecutionProcess::find_by_commander_session_id(pool, commander_session.id).await?;

    Ok(ResponseJson(ApiResponse::success(processes)))
}

/// Send a follow-up message to the commander
pub async fn follow_up(
    Extension(_commander_session): Extension<CommanderSession>,
    State(_deployment): State<DeploymentImpl>,
    Json(_payload): Json<CreateFollowUpRequest>,
) -> Result<ResponseJson<ApiResponse<ExecutionProcess>>, ApiError> {
    // Commander works directly in the main repo (no worktree)
    // The container_ref will be set to the repo path when execution starts

    // TODO: Start execution process for commander
    // - Get repo path from project
    // - Create ExecutionProcess with commander_session_id
    // - Start Claude Code in repo path with commander's system_prompt
    Err(ApiError::BadRequest(
        "Commander execution not yet implemented".to_string(),
    ))
}

/// Middleware to load commander session from path parameter
pub async fn load_commander_session_middleware(
    State(deployment): State<DeploymentImpl>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, ApiError> {
    let path = request.uri().path();

    // Extract commander_session_id from path like /api/commander/{commander_session_id}/...
    let session_id = path
        .split('/')
        .find_map(|segment| Uuid::parse_str(segment).ok())
        .ok_or_else(|| ApiError::BadRequest("Invalid commander session ID".to_string()))?;

    let pool = &deployment.db().pool;
    let session = CommanderSession::find_by_id(pool, session_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Commander session not found".to_string()))?;

    request.extensions_mut().insert(session);

    Ok(next.run(request).await)
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes that require a commander session
    let session_routes = Router::new()
        .route("/", get(get_commander))
        .route("/processes", get(get_commander_processes))
        .route("/follow-up", post(follow_up))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_commander_session_middleware,
        ));

    // Project-scoped routes for getting/creating commander
    let project_commander_routes = Router::new()
        .route("/commander", get(get_or_create_commander))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new()
        .nest("/projects/{project_id}", project_commander_routes)
        .nest("/commander/{commander_session_id}", session_routes)
}
