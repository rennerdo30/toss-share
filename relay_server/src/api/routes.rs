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
        .route("/api/health", get(health_check))
        // Device registration
        .route("/api/register", post(handlers::register_device))
        .route("/api/v1/register", post(handlers::register_device))
        .route("/api/v1/register", delete(handlers::unregister_device))
        // Message relay (Axum 0.8 uses {param} instead of :param)
        .route("/api/v1/relay/{device_id}", post(handlers::relay_message))
        // Device status
        .route(
            "/api/v1/devices/{device_id}/status",
            get(handlers::device_status),
        )
        // Pairing
        .route("/api/v1/pairing/register", post(handlers::register_pairing))
        .route("/api/v1/pairing/find/{code}", get(handlers::find_pairing))
        .route("/api/v1/pairing/{code}", delete(handlers::cancel_pairing))
        // WebSocket
        .route("/api/v1/ws", get(websocket::ws_handler))
}

async fn health_check() -> &'static str {
    "OK"
}
