use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::state::{Project, Task, TaskStatus};

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
