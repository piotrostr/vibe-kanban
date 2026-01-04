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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearIssue {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    #[serde(rename = "assignedIssues")]
    assigned_issues: IssueConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<LinearIssue>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
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

    /// Fetch all issues assigned to the current user that are in "backlog" state
    pub async fn fetch_backlog_issues(&self) -> Result<Vec<LinearIssue>, LinearError> {
        let query = r#"
            query {
                viewer {
                    assignedIssues(filter: { state: { type: { eq: "backlog" } } }) {
                        nodes {
                            id
                            title
                            description
                        }
                    }
                }
            }
        "#;

        let response = self
            .http
            .post(Self::API_URL)
            .header("Authorization", &self.api_key)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

        let result: GraphQLResponse = response.json().await?;

        if let Some(errors) = result.errors {
            let msg = errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(LinearError::Api(msg));
        }

        Ok(result
            .data
            .map(|d| d.viewer.assigned_issues.nodes)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_response() {
        let json = r#"{
            "data": {
                "viewer": {
                    "assignedIssues": {
                        "nodes": [
                            {
                                "id": "abc123",
                                "title": "Test Issue",
                                "description": "Some description"
                            }
                        ]
                    }
                }
            }
        }"#;

        let response: GraphQLResponse = serde_json::from_str(json).unwrap();
        let issues = response.data.unwrap().viewer.assigned_issues.nodes;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "abc123");
        assert_eq!(issues[0].title, "Test Issue");
    }
}
