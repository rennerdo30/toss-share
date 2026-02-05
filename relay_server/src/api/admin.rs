//! Admin dashboard handlers

use askama::Template;
use axum::{
    extract::{Path, State},
    http::{header::COOKIE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json, Redirect},
};
use chrono::{TimeZone, Utc};
use serde::Serialize;

use super::admin_auth::require_auth;
use crate::AppState;

/// Device view for templates
struct DeviceView {
    id: String,
    device_name: String,
    is_online: bool,
    last_seen: Option<String>,
    created_at: String,
}

/// Dashboard template
#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct DashboardTemplate {
    online_devices: i64,
    total_devices: i64,
    queued_messages: i64,
    uptime: String,
    active_connections: i64,
    connection_percent: i64,
}

/// Devices list template
#[derive(Template)]
#[template(path = "admin/devices.html")]
struct DevicesTemplate {
    devices: Vec<DeviceView>,
}

/// Stats response for API
#[derive(Serialize)]
pub struct ServerStats {
    online_devices: i64,
    total_devices: i64,
    queued_messages: i64,
    active_connections: usize,
    uptime_seconds: u64,
    pairing_sessions: i64,
}

/// Dashboard page handler
pub async fn dashboard(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Get stats
    let (online_devices, total_devices) =
        state.db.count_devices_by_status().await.unwrap_or((0, 0));

    let queued_messages = state.db.count_queued_messages().await.unwrap_or(0);
    let active_connections = state.relay.connection_count() as i64;
    let uptime = format_uptime(state.started_at.elapsed().as_secs());
    let connection_percent = if total_devices > 0 {
        (active_connections * 100) / total_devices
    } else {
        0
    };

    let template = DashboardTemplate {
        online_devices,
        total_devices,
        queued_messages,
        uptime,
        active_connections,
        connection_percent,
    };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Devices list page handler
pub async fn devices_list(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Get devices
    let devices = state.db.list_all_devices().await.unwrap_or_default();

    let device_views: Vec<DeviceView> = devices
        .into_iter()
        .map(|d| DeviceView {
            id: d.id,
            device_name: d.device_name,
            is_online: d.is_online,
            last_seen: d.last_seen.map(format_timestamp),
            created_at: format_timestamp(d.created_at),
        })
        .collect();

    let template = DevicesTemplate {
        devices: device_views,
    };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Delete a device
pub async fn delete_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Delete the device
    let _ = state.db.delete_device(&device_id).await;

    Redirect::to("/admin/devices").into_response()
}

/// Cleanup old messages
pub async fn cleanup_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Cleanup messages older than 24 hours
    let _ = state.db.cleanup_old_messages(86400).await;

    Redirect::to("/admin").into_response()
}

/// Cleanup expired pairings
pub async fn cleanup_pairings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Cleanup expired pairings
    let _ = state.db.cleanup_expired_pairings().await;

    Redirect::to("/admin").into_response()
}

/// Cleanup stale devices
pub async fn cleanup_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Cleanup devices not seen in 30 days
    let _ = state.db.cleanup_stale_devices(30).await;

    Redirect::to("/admin").into_response()
}

/// Get server stats as JSON (API endpoint)
pub async fn get_stats(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Admin not configured"})),
        )
            .into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if !super::admin_auth::is_authenticated(cookies.unwrap_or(""), &state.config.session_secret) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    // Get stats
    let (online_devices, total_devices) =
        state.db.count_devices_by_status().await.unwrap_or((0, 0));

    let queued_messages = state.db.count_queued_messages().await.unwrap_or(0);
    let pairing_sessions = state.db.count_pairing_sessions().await.unwrap_or(0);
    let active_connections = state.relay.connection_count();
    let uptime_seconds = state.started_at.elapsed().as_secs();

    let stats = ServerStats {
        online_devices,
        total_devices,
        queued_messages,
        active_connections,
        uptime_seconds,
        pairing_sessions,
    };

    Json(stats).into_response()
}

/// Format uptime duration
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Format a Unix timestamp to readable string
fn format_timestamp(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}
