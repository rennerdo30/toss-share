//! Toss Relay Server Library
//!
//! This module exposes the relay server components for testing and embedding.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod relay;

pub use config::Config;
pub use db::Database;
pub use relay::RelayState;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<Database>,
    pub relay: Arc<RelayState>,
}

/// Create the application router with the given state
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .merge(api::routes::create_router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Test server handle for integration tests
pub struct TestServer {
    pub addr: SocketAddr,
    pub base_url: String,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Start a test server on a random available port
    pub async fn start() -> anyhow::Result<Self> {
        Self::start_with_config(Config::default()).await
    }

    /// Start a test server with custom configuration
    pub async fn start_with_config(mut config: Config) -> anyhow::Result<Self> {
        // Use port 0 to get a random available port
        config.port = 0;
        // Use in-memory SQLite for tests
        config.database_url = "sqlite::memory:".to_string();

        // Initialize database
        let database = Database::new(&config.database_url).await?;
        database.migrate().await?;

        // Create application state
        let state = AppState {
            config: Arc::new(config),
            db: Arc::new(database),
            relay: Arc::new(RelayState::new()),
        };

        // Create the app
        let app = create_app(state);

        // Bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{}", addr);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn the server
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        Ok(Self {
            addr,
            base_url,
            shutdown_tx,
            handle,
        })
    }

    /// Get the server's base URL
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Shutdown the test server
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.handle.await;
    }
}
