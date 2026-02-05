//! API route definitions

use axum::{
    routing::{delete, get, post},
    Router,
};

use super::{admin, admin_auth, handlers, websocket};
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
        // Admin dashboard
        .route("/admin", get(admin::dashboard))
        .route("/admin/login", get(admin_auth::login_page))
        .route("/admin/login", post(admin_auth::login_submit))
        .route("/admin/logout", post(admin_auth::logout))
        .route("/admin/devices", get(admin::devices_list))
        .route(
            "/admin/devices/{device_id}/delete",
            post(admin::delete_device),
        )
        .route("/admin/cleanup/messages", post(admin::cleanup_messages))
        .route("/admin/cleanup/pairings", post(admin::cleanup_pairings))
        .route("/admin/cleanup/devices", post(admin::cleanup_devices))
        .route("/admin/sessions", get(admin::sessions_list))
        .route("/admin/sessions/{code}/cancel", post(admin::cancel_session))
        .route(
            "/admin/devices/{device_id}/disconnect",
            post(admin::disconnect_device),
        )
        .route("/admin/logs", get(admin::logs_page))
        .route("/admin/logs/clear", post(admin::clear_logs))
        .route("/api/admin/stats", get(admin::get_stats))
}

async fn health_check() -> &'static str {
    "OK"
}
