//! Toss Relay Server
//!
//! A relay server for Toss that enables clipboard sync between devices
//! when direct P2P connections are not possible.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use toss_relay::{create_app, AppState, Config, Database, RelayState};

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
    let app = create_app(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("Server ready");
    axum::serve(listener, app).await?;

    Ok(())
}
