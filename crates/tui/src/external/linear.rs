use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LinearIssue {
    pub identifier: String, // Human-readable ID like "VIB-6"
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
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
struct IssueConnection {
    nodes: Vec<IssueNode>,
}

#[derive(Debug, Deserialize)]
struct IssueNode {
    identifier: String,
    title: String,
    description: Option<String>,
    url: String,
    labels: Option<LabelConnection>,
}

#[derive(Debug, Deserialize)]
struct LabelConnection {
    nodes: Vec<LabelNode>,
}

#[derive(Debug, Deserialize)]
struct LabelNode {
    name: String,
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

    /// Fetch backlog issues assigned to the current user (API key owner)
    pub async fn fetch_backlog_issues(&self) -> Result<Vec<LinearIssue>, String> {
        let query = r#"
            query {
                viewer {
                    assignedIssues(filter: { state: { type: { eq: "backlog" } } }) {
                        nodes {
                            identifier
                            title
                            description
                            url
                            labels {
                                nodes {
                                    name
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let body = serde_json::json!({ "query": query });

        let response = self
            .http
            .post(Self::API_URL)
            .header("Authorization", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            ));
        }

        let result: GraphQLResponse<ViewerData> = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(errors) = result.errors {
            let msg = errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("GraphQL error: {}", msg));
        }

        let data = result.data.ok_or("No data in response")?;
        let issues = data
            .viewer
            .assigned_issues
            .map(|c| c.nodes)
            .unwrap_or_default();

        Ok(issues
            .into_iter()
            .map(|node| LinearIssue {
                identifier: node.identifier,
                title: node.title,
                description: node.description,
                url: node.url,
                labels: node
                    .labels
                    .map(|l| l.nodes.into_iter().map(|n| n.name).collect())
                    .unwrap_or_default(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_api_key() -> Option<String> {
        std::env::var("VIBE_KANBAN_LINEAR_API_KEY").ok()
    }

    #[tokio::test]
    async fn test_fetch_backlog_issues() {
        let Some(api_key) = get_test_api_key() else {
            eprintln!("Skipping test: VIBE_KANBAN_LINEAR_API_KEY not set");
            return;
        };

        let client = LinearClient::new(api_key);
        let result = client.fetch_backlog_issues().await;

        match result {
            Ok(issues) => {
                println!("Found {} assigned backlog issues", issues.len());
                for issue in &issues {
                    println!("  - {} [{}]", issue.title, issue.identifier);
                }
            }
            Err(e) => {
                panic!("Failed to fetch issues: {}", e);
            }
        }
    }
}
