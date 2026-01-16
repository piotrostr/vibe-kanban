use crate::AppState;
use server::{ServerConfig, run};
use std::time::Duration;
use tauri::{AppHandle, Manager, Url};
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::port_file::read_port_file;

pub async fn spawn_server(app: AppHandle) {
    init_tracing();

    let state = app.state::<AppState>();

    tokio::spawn(async move {
        let config = ServerConfig {
            skip_browser_open: true,
        };

        match run(config).await {
            Ok(port) => {
                tracing::info!("Server exited normally, was on port {}", port);
            }
            Err(e) => {
                tracing::error!("Server failed: {}", e);
            }
        }
    });

    match wait_for_server_ready().await {
        Ok(port) => {
            *state.server_port.write().await = Some(port);
            tracing::info!("Server ready on port {}", port);

            if let Some(window) = app.get_webview_window("main") {
                let url = format!("http://127.0.0.1:{}", port);
                if let Ok(parsed) = url.parse::<Url>() {
                    if let Err(e) = window.navigate(parsed) {
                        tracing::error!("Failed to navigate window: {}", e);
                    }
                    if let Err(e) = window.show() {
                        tracing::error!("Failed to show window: {}", e);
                    }
                }
            }

            crate::tray::update_status(&app, true);
        }
        Err(e) => {
            tracing::error!("Server failed to start: {}", e);
        }
    }
}

async fn wait_for_server_ready() -> Result<u16, String> {
    let max_attempts = 100;
    let delay = Duration::from_millis(100);

    for attempt in 0..max_attempts {
        if let Ok(port) = read_port_file("vibe").await {
            let health_url = format!("http://127.0.0.1:{}/api/health", port);
            if let Ok(resp) = reqwest::get(&health_url).await {
                if resp.status().is_success() {
                    tracing::info!("Server health check passed on attempt {}", attempt + 1);
                    return Ok(port);
                }
            }
        }
        tokio::time::sleep(delay).await;
    }

    Err("Server failed to start within timeout".to_string())
}

fn init_tracing() {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter_string = format!(
        "warn,vibe_kanban_desktop_lib={level},server={level},services={level},db={level},executors={level},deployment={level},local_deployment={level},utils={level}",
        level = log_level
    );

    if let Ok(env_filter) = EnvFilter::try_new(filter_string) {
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .try_init();
    }
}
