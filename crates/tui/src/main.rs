use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod api;
mod external;
mod input;
mod state;
mod terminal;
mod ui;

use app::App;
use terminal::Terminal;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let port = discover_backend_port().await?;
    tracing::info!("Connecting to backend on port {}", port);

    let mut terminal = Terminal::new()?;
    let mut app = App::new(port).await?;

    let result = app.run(&mut terminal).await;

    terminal.restore()?;

    result
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,tui=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

async fn discover_backend_port() -> Result<u16> {
    // Check environment variable first
    if let Ok(port_str) = std::env::var("VIBE_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            return Ok(port);
        }
    }

    // Try to read port from the port file (same mechanism as Tauri)
    if let Ok(port) = utils::port_file::read_port_file("vibe").await {
        // Verify the server is running
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/api/health", port);

        if client.get(&health_url).send().await.is_ok() {
            return Ok(port);
        }
    }

    // Fallback: try common development ports
    for port in [6770, 3000, 8080] {
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/api/health", port);

        if client.get(&health_url).send().await.is_ok() {
            return Ok(port);
        }
    }

    anyhow::bail!(
        "Could not find running backend. Start it with: pnpm run backend:dev\n\
         Or specify port with: VIBE_PORT=<port> vibe"
    )
}
