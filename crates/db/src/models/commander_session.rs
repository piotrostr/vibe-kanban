use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

pub const DEFAULT_COMMANDER_SYSTEM_PROMPT: &str = r#"You are Commander, a repository operator for this project.

RULES:
- NEVER push directly to main/master branches
- NEVER force push to any branch
- Always use PRs for merging changes to main
- For consolidating changes across tickets, create a new branch first
- Use `gh pr merge` for merging approved PRs
- Before any destructive git operation, explain what you are about to do and ask for confirmation

You have access to the vibe-kanban MCP for task management."#;

#[derive(Debug, Error)]
pub enum CommanderSessionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Commander session not found")]
    NotFound,
    #[error("Project not found")]
    ProjectNotFound,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct CommanderSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub container_ref: Option<String>,
    pub executor: Option<String>,
    pub system_prompt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCommanderSession {
    pub executor: Option<String>,
    pub system_prompt: Option<String>,
}

impl CommanderSession {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CommanderSession,
            r#"SELECT id AS "id!: Uuid",
                      project_id AS "project_id!: Uuid",
                      container_ref,
                      executor,
                      system_prompt,
                      created_at AS "created_at!: DateTime<Utc>",
                      updated_at AS "updated_at!: DateTime<Utc>"
               FROM commander_sessions
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CommanderSession,
            r#"SELECT id AS "id!: Uuid",
                      project_id AS "project_id!: Uuid",
                      container_ref,
                      executor,
                      system_prompt,
                      created_at AS "created_at!: DateTime<Utc>",
                      updated_at AS "updated_at!: DateTime<Utc>"
               FROM commander_sessions
               WHERE project_id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateCommanderSession,
        id: Uuid,
        project_id: Uuid,
    ) -> Result<Self, CommanderSessionError> {
        let system_prompt = data
            .system_prompt
            .as_deref()
            .unwrap_or(DEFAULT_COMMANDER_SYSTEM_PROMPT);

        Ok(sqlx::query_as!(
            CommanderSession,
            r#"INSERT INTO commander_sessions (id, project_id, executor, system_prompt)
               VALUES ($1, $2, $3, $4)
               RETURNING id AS "id!: Uuid",
                         project_id AS "project_id!: Uuid",
                         container_ref,
                         executor,
                         system_prompt,
                         created_at AS "created_at!: DateTime<Utc>",
                         updated_at AS "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.executor,
            system_prompt
        )
        .fetch_one(pool)
        .await?)
    }

    /// Find or create a commander session for a project
    pub async fn find_or_create(
        pool: &SqlitePool,
        project_id: Uuid,
        executor: Option<&str>,
    ) -> Result<Self, CommanderSessionError> {
        if let Some(session) = Self::find_by_project_id(pool, project_id).await? {
            return Ok(session);
        }

        let data = CreateCommanderSession {
            executor: executor.map(|s| s.to_string()),
            system_prompt: None,
        };
        Self::create(pool, &data, Uuid::new_v4(), project_id).await
    }

    pub async fn update_container_ref(
        pool: &SqlitePool,
        id: Uuid,
        container_ref: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE commander_sessions
               SET container_ref = $1, updated_at = datetime('now', 'subsec')
               WHERE id = $2"#,
            container_ref,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_system_prompt(
        pool: &SqlitePool,
        id: Uuid,
        system_prompt: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE commander_sessions
               SET system_prompt = $1, updated_at = datetime('now', 'subsec')
               WHERE id = $2"#,
            system_prompt,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
