use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    actions::Executable,
    approvals::ExecutorApprovalService,
    env::ExecutionEnv,
    executors::{BaseCodingAgent, ExecutorError, SpawnedChild, StandardCodingAgentExecutor},
    mcp_config::{ensure_mcps_in_config, McpApiKeys},
    profile::{ExecutorConfigs, ExecutorProfileId},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct CodingAgentFollowUpRequest {
    pub prompt: String,
    pub session_id: String,
    /// Executor profile specification
    #[serde(alias = "profile_variant_label")]
    // Backwards compatability with ProfileVariantIds, esp stored in DB under ExecutorAction
    pub executor_profile_id: ExecutorProfileId,
    /// Optional relative path to execute the agent in (relative to container_ref).
    /// If None, uses the container_ref directory directly.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// List of MCP server keys to enable for this follow-up (e.g., ["linear", "sentry"])
    #[serde(default)]
    pub enabled_mcps: Option<Vec<String>>,
    /// API keys for MCP integrations (not serialized - sensitive data)
    #[serde(skip)]
    #[ts(skip)]
    pub mcp_api_keys: McpApiKeys,
}

impl CodingAgentFollowUpRequest {
    /// Get the executor profile ID
    pub fn get_executor_profile_id(&self) -> ExecutorProfileId {
        self.executor_profile_id.clone()
    }

    pub fn base_executor(&self) -> BaseCodingAgent {
        self.executor_profile_id.executor
    }
}

#[async_trait]
impl Executable for CodingAgentFollowUpRequest {
    async fn spawn(
        &self,
        current_dir: &Path,
        approvals: Arc<dyn ExecutorApprovalService>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        let effective_dir = match &self.working_dir {
            Some(rel_path) => current_dir.join(rel_path),
            None => current_dir.to_path_buf(),
        };

        let executor_profile_id = self.get_executor_profile_id();
        let mut agent = ExecutorConfigs::get_cached()
            .get_coding_agent(&executor_profile_id)
            .ok_or(ExecutorError::UnknownExecutorType(
                executor_profile_id.to_string(),
            ))?;

        // Inject enabled MCPs into agent config before spawning
        if let Some(ref enabled_mcps) = self.enabled_mcps {
            if let Err(e) = ensure_mcps_in_config(&agent, enabled_mcps, &self.mcp_api_keys).await {
                tracing::warn!(
                    error = %e,
                    mcps = ?enabled_mcps,
                    "Failed to inject MCPs into agent config, continuing anyway"
                );
            }
        }

        agent.use_approvals(approvals.clone());

        agent
            .spawn_follow_up(&effective_dir, &self.prompt, &self.session_id, env)
            .await
    }
}
