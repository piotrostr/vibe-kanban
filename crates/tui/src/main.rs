use anyhow::Result;
use std::fs::OpenOptions;
use std::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod app;
mod embedded_server;
mod external;
mod input;
mod state;
mod terminal;
mod ui;

use app::App;
use embedded_server::EmbeddedServer;
use terminal::Terminal;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let server = EmbeddedServer::start().await?;
    let port = server.port();
    tracing::info!("Using embedded server on port {}", port);

    let mut terminal = Terminal::new()?;
    let mut app = App::new(port).await?;

    let result = app.run(&mut terminal).await;

    terminal.restore()?;

    drop(server);

    result
}

fn init_tracing() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,tui=info,server=info"));

    // Write logs to file instead of stderr to avoid breaking TUI
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".vibe");
    std::fs::create_dir_all(&log_dir)?;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("vibe.log"))?;

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(Mutex::new(log_file)))
        .init();

    Ok(())
}
