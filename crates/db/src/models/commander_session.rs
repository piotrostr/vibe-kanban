use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

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
    pub branch: String,
    pub executor: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateCommanderSession {
    pub branch: String,
    pub executor: Option<String>,
}

impl CommanderSession {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CommanderSession,
            r#"SELECT id AS "id!: Uuid",
                      project_id AS "project_id!: Uuid",
                      container_ref,
                      branch,
                      executor,
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
                      branch,
                      executor,
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
        Ok(sqlx::query_as!(
            CommanderSession,
            r#"INSERT INTO commander_sessions (id, project_id, branch, executor)
               VALUES ($1, $2, $3, $4)
               RETURNING id AS "id!: Uuid",
                         project_id AS "project_id!: Uuid",
                         container_ref,
                         branch,
                         executor,
                         created_at AS "created_at!: DateTime<Utc>",
                         updated_at AS "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.branch,
            data.executor
        )
        .fetch_one(pool)
        .await?)
    }

    /// Find or create a commander session for a project
    pub async fn find_or_create(
        pool: &SqlitePool,
        project_id: Uuid,
        branch: &str,
        executor: Option<&str>,
    ) -> Result<Self, CommanderSessionError> {
        if let Some(session) = Self::find_by_project_id(pool, project_id).await? {
            return Ok(session);
        }

        let data = CreateCommanderSession {
            branch: branch.to_string(),
            executor: executor.map(|s| s.to_string()),
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
}
