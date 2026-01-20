use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::state::{ExecutionProcess, Project, Session, Task, TaskStatus, Workspace};

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error_data: Option<serde_json::Value>,
    pub message: Option<String>,
}

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response: ApiResponse<T> = self.client.get(&url).send().await?.json().await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>> {
        self.get("/api/projects").await
    }

    pub async fn get_tasks(&self, project_id: &str) -> Result<Vec<Task>> {
        self.get(&format!("/api/tasks?project_id={}", project_id))
            .await
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/api/health", self.base_url);
        self.client.get(&url).send().await?;
        Ok(())
    }

    pub fn websocket_url(&self, path: &str) -> String {
        let ws_base = self.base_url.replace("http://", "ws://");
        format!("{}{}", ws_base, path)
    }

    pub async fn update_task(&self, task_id: &str, update: UpdateTask) -> Result<Task> {
        let url = format!("{}/api/tasks/{}", self.base_url, task_id);
        let response: ApiResponse<Task> = self
            .client
            .put(&url)
            .json(&update)
            .send()
            .await?
            .json()
            .await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        let url = format!("{}/api/tasks/{}", self.base_url, task_id);
        let response: ApiResponse<()> = self.client.delete(&url).send().await?.json().await?;

        if response.success {
            Ok(())
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    pub async fn create_task(&self, create: CreateTask) -> Result<Task> {
        let url = format!("{}/api/tasks", self.base_url);
        let response: ApiResponse<Task> = self
            .client
            .post(&url)
            .json(&create)
            .send()
            .await?
            .json()
            .await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    // Attempt/Workspace methods
    pub async fn get_task_attempts(&self, task_id: &str) -> Result<Vec<Workspace>> {
        self.get(&format!("/api/task-attempts?task_id={}", task_id))
            .await
    }

    pub async fn create_task_attempt(&self, create: CreateTaskAttempt) -> Result<Workspace> {
        let url = format!("{}/api/task-attempts", self.base_url);
        let response: ApiResponse<Workspace> = self
            .client
            .post(&url)
            .json(&create)
            .send()
            .await?
            .json()
            .await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    // Session methods
    pub async fn get_sessions(&self, workspace_id: &str) -> Result<Vec<Session>> {
        self.get(&format!("/api/sessions?workspace_id={}", workspace_id))
            .await
    }

    pub async fn create_session(&self, create: CreateSession) -> Result<Session> {
        let url = format!("{}/api/sessions", self.base_url);
        let response: ApiResponse<Session> = self
            .client
            .post(&url)
            .json(&create)
            .send()
            .await?
            .json()
            .await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }

    pub async fn send_follow_up(
        &self,
        session_id: &str,
        follow_up: FollowUpRequest,
    ) -> Result<ExecutionProcess> {
        let url = format!("{}/api/sessions/{}/follow-up", self.base_url, session_id);
        let response: ApiResponse<ExecutionProcess> = self
            .client
            .post(&url)
            .json(&follow_up)
            .send()
            .await?
            .json()
            .await?;

        if response.success {
            response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            anyhow::bail!(
                "API error: {}",
                response.message.unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UpdateTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub sync_to_linear: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateTask {
    pub project_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceRepoInput {
    pub repo_id: String,
    pub target_branch: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTaskAttempt {
    pub task_id: String,
    pub executor_profile_id: ExecutorProfileId,
    pub repos: Vec<WorkspaceRepoInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutorProfileId {
    pub executor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSession {
    pub workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FollowUpRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}
