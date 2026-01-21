use anyhow::{Context, Result};
use server::EmbeddedServerHandle;
use std::time::Duration;

pub struct EmbeddedServer {
    handle: EmbeddedServerHandle,
}

impl EmbeddedServer {
    pub async fn start() -> Result<Self> {
        tracing::info!("Starting embedded server...");

        let handle = server::run_embedded()
            .await
            .context("Failed to start embedded server")?;

        let port = handle.port();
        tracing::info!("Embedded server bound to port {}", port);

        Self::wait_for_ready(port).await?;

        Ok(Self { handle })
    }

    pub fn port(&self) -> u16 {
        self.handle.port()
    }

    async fn wait_for_ready(port: u16) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let health_url = format!("http://127.0.0.1:{}/api/health", port);

        for attempt in 1..=20 {
            match client.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("Server ready after {} attempts", attempt);
                    return Ok(());
                }
                Ok(resp) => {
                    tracing::debug!("Health check returned status {}", resp.status());
                }
                Err(e) => {
                    tracing::debug!("Health check attempt {} failed: {}", attempt, e);
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("Server failed to become ready within 2 seconds")
    }
}
