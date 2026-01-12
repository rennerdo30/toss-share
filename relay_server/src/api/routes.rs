//! API route definitions

use axum::{
    routing::{delete, get, post},
    Router,
};

use super::{handlers, websocket};
use crate::AppState;

/// Create the API router
pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Device registration
        .route("/api/v1/register", post(handlers::register_device))
        .route("/api/v1/register", delete(handlers::unregister_device))
        // Message relay
        .route("/api/v1/relay/:device_id", post(handlers::relay_message))
        // Device status
        .route(
            "/api/v1/devices/:device_id/status",
            get(handlers::device_status),
        )
        // WebSocket
        .route("/api/v1/ws", get(websocket::ws_handler))
}

async fn health_check() -> &'static str {
    "OK"
}
