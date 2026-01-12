//! Toss Relay Server
//!
//! A relay server for Toss that enables clipboard sync between devices
//! when direct P2P connections are not possible.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod config;
mod db;
mod error;
mod relay;

use config::Config;
use db::Database;
use relay::RelayState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "toss_relay=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    tracing::info!("Starting Toss Relay Server");
    tracing::info!("Listening on {}:{}", config.host, config.port);

    // Initialize database
    let database = Database::new(&config.database_url).await?;
    database.migrate().await?;

    // Create application state
    let state = AppState {
        config: Arc::new(config.clone()),
        db: Arc::new(database),
        relay: Arc::new(RelayState::new()),
    };

    // Build router
    let app = Router::new()
        .merge(api::routes::create_router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("Server ready");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<Database>,
    pub relay: Arc<RelayState>,
}
