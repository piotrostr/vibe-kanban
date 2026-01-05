mod app;
mod auth;
pub mod config;
pub mod db;
pub mod github_app;
pub mod mail;
pub mod r2;
pub mod routes;
mod state;
pub mod validated_where;

use std::env;

pub use app::Server;
pub use state::AppState;
use tracing_error::ErrorLayer;

/// Configure Sentry user scope (no-op after analytics removal)
pub fn configure_user_scope(_user_id: uuid::Uuid, _username: Option<&str>, _email: Option<&str>) {
    // Analytics/Sentry user scope configuration removed
}
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
};

pub fn init_tracing() {
    if tracing::dispatcher::has_been_set() {
        return;
    }

    let env_filter = env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".to_string());
    let fmt_layer = fmt::layer()
        .json()
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .boxed();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(env_filter))
        .with(ErrorLayer::default())
        .with(fmt_layer)
        .init();
}
