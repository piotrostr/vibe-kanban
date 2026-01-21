use anyhow::Result;
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
    init_tracing();

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

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,tui=info,server=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}
