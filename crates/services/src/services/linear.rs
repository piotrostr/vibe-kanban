use db::models::task::TaskStatus;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinearError {
    #[error("network error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Linear API error: {0}")]
    Api(String),
    #[error("missing API key")]
    MissingApiKey,
    #[error("state not found: {0}")]
    StateNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
}

/// Workflow state in Linear (e.g., Backlog, Todo, In Progress, Done)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String, // "backlog", "unstarted", "started", "completed", "cancelled"
}

/// Linear user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearUser {
    pub id: String,
    pub name: String,
}

// Response types for different GraphQL queries
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct ViewerData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    #[serde(rename = "assignedIssues")]
    assigned_issues: Option<IssueConnection>,
}

#[derive(Debug, Deserialize)]
struct IssuesData {
    issues: IssueConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<LinearIssue>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStatesData {
    #[serde(rename = "workflowStates")]
    workflow_states: WorkflowStateConnection,
}

#[derive(Debug, Deserialize)]
struct WorkflowStateConnection {
    nodes: Vec<WorkflowState>,
}

#[derive(Debug, Deserialize)]
struct UserData {
    user: Option<LinearUser>,
}

#[derive(Debug, Deserialize)]
struct IssueUpdateData {
    #[serde(rename = "issueUpdate")]
    issue_update: IssueUpdateResult,
}

#[derive(Debug, Deserialize)]
struct IssueUpdateResult {
    success: bool,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

/// Map local TaskStatus to Linear state type
pub fn task_status_to_linear_state_type(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Backlog => "backlog",
        TaskStatus::Todo => "unstarted",
        TaskStatus::InProgress => "started",
        TaskStatus::InReview => "started", // Linear doesn't have a review state
        TaskStatus::Done => "completed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Map Linear state type to local TaskStatus
pub fn linear_state_type_to_task_status(state_type: &str) -> TaskStatus {
    match state_type {
        "backlog" => TaskStatus::Backlog,
        "unstarted" => TaskStatus::Todo,
        "started" => TaskStatus::InProgress,
        "completed" => TaskStatus::Done,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Backlog, // Default fallback
    }
}

pub struct LinearClient {
    http: Client,
    api_key: String,
}

impl LinearClient {
    const API_URL: &'static str = "https://api.linear.app/graphql";

    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
        }
    }

    /// Execute a GraphQL query and handle common response patterns
    async fn execute_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<T, LinearError> {
        let body = match variables {
            Some(vars) => serde_json::json!({ "query": query, "variables": vars }),
            None => serde_json::json!({ "query": query }),
        };

        let response = self
            .http
            .post(Self::API_URL)
            .header("Authorization", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(LinearError::Api(format!(
                "HTTP {} - {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            )));
        }

        let result: GraphQLResponse<T> = response.json().await?;

        if let Some(errors) = result.errors {
            let msg = errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(LinearError::Api(msg));
        }

        result
            .data
            .ok_or_else(|| LinearError::Api("No data in response".to_string()))
    }

    /// Fetch all issues assigned to the current user (viewer) that are in "backlog" state
    pub async fn fetch_backlog_issues(&self) -> Result<Vec<LinearIssue>, LinearError> {
        let query = r#"
            query {
                viewer {
                    assignedIssues(filter: { state: { type: { eq: "backlog" } } }) {
                        nodes {
                            id
                            title
                            description
                            url
                        }
                    }
                }
            }
        "#;

        let data: ViewerData = self.execute_query(query, None).await?;
        Ok(data
            .viewer
            .assigned_issues
            .map(|c| c.nodes)
            .unwrap_or_default())
    }

    /// Fetch issues filtered by assignee ID and optionally by state type
    pub async fn fetch_issues_by_assignee(
        &self,
        assignee_id: &str,
        state_type: Option<&str>,
    ) -> Result<Vec<LinearIssue>, LinearError> {
        let filter = match state_type {
            Some(st) => format!(
                r#"{{ assignee: {{ id: {{ eq: "{}" }} }}, state: {{ type: {{ eq: "{}" }} }} }}"#,
                assignee_id, st
            ),
            None => format!(r#"{{ assignee: {{ id: {{ eq: "{}" }} }} }}"#, assignee_id),
        };

        let query = format!(
            r#"
            query {{
                issues(filter: {}) {{
                    nodes {{
                        id
                        title
                        description
                        url
                    }}
                }}
            }}
        "#,
            filter
        );

        let data: IssuesData = self.execute_query(&query, None).await?;
        Ok(data.issues.nodes)
    }

    /// Fetch all workflow states available in the organization
    pub async fn fetch_workflow_states(&self) -> Result<Vec<WorkflowState>, LinearError> {
        let query = r#"
            query {
                workflowStates {
                    nodes {
                        id
                        name
                        type
                    }
                }
            }
        "#;

        let data: WorkflowStatesData = self.execute_query(query, None).await?;
        Ok(data.workflow_states.nodes)
    }

    /// Validate that a user exists in Linear
    pub async fn validate_user(&self, user_id: &str) -> Result<Option<LinearUser>, LinearError> {
        let query = r#"
            query($id: String!) {
                user(id: $id) {
                    id
                    name
                }
            }
        "#;

        let variables = serde_json::json!({ "id": user_id });
        let data: UserData = self.execute_query(query, Some(variables)).await?;
        Ok(data.user)
    }

    /// Update an issue's state in Linear
    pub async fn update_issue_state(
        &self,
        issue_id: &str,
        state_id: &str,
    ) -> Result<(), LinearError> {
        let query = r#"
            mutation($issueId: String!, $stateId: String!) {
                issueUpdate(id: $issueId, input: { stateId: $stateId }) {
                    success
                }
            }
        "#;

        let variables = serde_json::json!({
            "issueId": issue_id,
            "stateId": state_id
        });

        let data: IssueUpdateData = self.execute_query(query, Some(variables)).await?;

        if !data.issue_update.success {
            return Err(LinearError::Api("Issue update failed".to_string()));
        }

        Ok(())
    }

    /// Update an issue's state in Linear using task status
    /// This fetches workflow states, finds the matching state, and updates the issue
    pub async fn sync_task_status_to_linear(
        &self,
        issue_id: &str,
        status: &TaskStatus,
    ) -> Result<(), LinearError> {
        let states = self.fetch_workflow_states().await?;
        let target_type = task_status_to_linear_state_type(status);

        let state = states
            .iter()
            .find(|s| s.state_type == target_type)
            .ok_or_else(|| LinearError::StateNotFound(target_type.to_string()))?;

        self.update_issue_state(issue_id, &state.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_viewer_response() {
        let json = r#"{
            "data": {
                "viewer": {
                    "assignedIssues": {
                        "nodes": [
                            {
                                "id": "abc123",
                                "title": "Test Issue",
                                "description": "Some description",
                                "url": "https://linear.app/team/issue/ABC-123"
                            }
                        ]
                    }
                }
            }
        }"#;

        let response: GraphQLResponse<ViewerData> = serde_json::from_str(json).unwrap();
        let issues = response
            .data
            .unwrap()
            .viewer
            .assigned_issues
            .unwrap()
            .nodes;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "abc123");
        assert_eq!(issues[0].title, "Test Issue");
        assert_eq!(issues[0].url, "https://linear.app/team/issue/ABC-123");
    }

    #[test]
    fn test_deserialize_workflow_states_response() {
        let json = r#"{
            "data": {
                "workflowStates": {
                    "nodes": [
                        { "id": "state1", "name": "Backlog", "type": "backlog" },
                        { "id": "state2", "name": "Todo", "type": "unstarted" },
                        { "id": "state3", "name": "In Progress", "type": "started" },
                        { "id": "state4", "name": "Done", "type": "completed" }
                    ]
                }
            }
        }"#;

        let response: GraphQLResponse<WorkflowStatesData> = serde_json::from_str(json).unwrap();
        let states = response.data.unwrap().workflow_states.nodes;
        assert_eq!(states.len(), 4);
        assert_eq!(states[0].state_type, "backlog");
        assert_eq!(states[3].state_type, "completed");
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            task_status_to_linear_state_type(&TaskStatus::Backlog),
            "backlog"
        );
        assert_eq!(
            task_status_to_linear_state_type(&TaskStatus::Todo),
            "unstarted"
        );
        assert_eq!(
            task_status_to_linear_state_type(&TaskStatus::InProgress),
            "started"
        );
        assert_eq!(
            task_status_to_linear_state_type(&TaskStatus::Done),
            "completed"
        );

        assert!(matches!(
            linear_state_type_to_task_status("backlog"),
            TaskStatus::Backlog
        ));
        assert!(matches!(
            linear_state_type_to_task_status("unstarted"),
            TaskStatus::Todo
        ));
        assert!(matches!(
            linear_state_type_to_task_status("started"),
            TaskStatus::InProgress
        ));
        assert!(matches!(
            linear_state_type_to_task_status("completed"),
            TaskStatus::Done
        ));
    }
}
