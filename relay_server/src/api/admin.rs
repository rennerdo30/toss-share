//! Admin dashboard handlers

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{header::COOKIE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json, Redirect},
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

use super::admin_auth::require_auth;
use crate::db::TeamRole;
use crate::AppState;

/// Maximum number of log entries to keep in memory
const MAX_LOG_ENTRIES: usize = 1000;

/// In-memory log storage for the admin dashboard
pub struct LogBuffer {
    entries: RwLock<VecDeque<LogEntry>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
        }
    }

    /// Add a new log entry
    pub fn push(&self, entry: LogEntry) {
        let mut entries = self.entries.write().unwrap();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Get all log entries, optionally filtered by level
    pub fn get_entries(&self, level_filter: Option<&str>) -> Vec<LogEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .filter(|e| {
                level_filter
                    .map(|f| {
                        if f == "all" {
                            true
                        } else {
                            e.level.to_lowercase() == f.to_lowercase()
                        }
                    })
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Clear all log entries
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global log buffer instance
pub static LOG_BUFFER: std::sync::LazyLock<LogBuffer> = std::sync::LazyLock::new(LogBuffer::new);

/// A single log entry
#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Log entry view for templates
struct LogEntryView {
    timestamp: String,
    level: String,
    target: String,
    message: String,
}

/// Device view for templates
struct DeviceView {
    id: String,
    device_name: String,
    is_online: bool,
    last_seen: Option<String>,
    created_at: String,
}

/// Session view for templates
struct SessionView {
    code: String,
    device_name: String,
    created_at: String,
    expires_at: String,
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
    memory_usage: String,
    pairing_sessions: i64,
    total_teams: i64,
}

/// Devices list template
#[derive(Template)]
#[template(path = "admin/devices.html")]
struct DevicesTemplate {
    devices: Vec<DeviceView>,
}

/// Sessions list template
#[derive(Template)]
#[template(path = "admin/sessions.html")]
struct SessionsTemplate {
    sessions: Vec<SessionView>,
}

/// Logs template
#[derive(Template)]
#[template(path = "admin/logs.html")]
struct LogsTemplate {
    logs: Vec<LogEntryView>,
    level: String,
}

/// Team view for templates
struct TeamView {
    id: String,
    name: String,
    description: Option<String>,
    member_count: i64,
    pending_invitations: i64,
    broadcast_enabled: bool,
    created_at: String,
}

/// Team member view for templates
struct TeamMemberView {
    device_id: String,
    device_name: String,
    is_admin: bool,
    joined_at: String,
}

/// Team invitation view for templates
struct TeamInvitationView {
    code: String,
    status: String,
    created_at: String,
    expires_at: String,
}

/// Team audit entry view for templates
struct TeamAuditEntryView {
    action: String,
    device_id: String,
    details: Option<String>,
    created_at: String,
}

/// Teams list template
#[derive(Template)]
#[template(path = "admin/teams.html")]
struct TeamsTemplate {
    teams: Vec<TeamView>,
}

/// Team details template
#[derive(Template)]
#[template(path = "admin/team_details.html")]
struct TeamDetailsTemplate {
    team: TeamView,
    members: Vec<TeamMemberView>,
    invitations: Vec<TeamInvitationView>,
    audit_entries: Vec<TeamAuditEntryView>,
}

/// Query parameters for logs page
#[derive(Deserialize)]
pub struct LogsQuery {
    level: Option<String>,
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
    memory_usage_bytes: u64,
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
    let pairing_sessions = state.db.count_pairing_sessions().await.unwrap_or(0);
    let total_teams = state.db.count_teams().await.unwrap_or(0);
    let active_connections = state.relay.connection_count() as i64;
    let uptime = format_uptime(state.started_at.elapsed().as_secs());
    let connection_percent = if total_devices > 0 {
        (active_connections * 100) / total_devices
    } else {
        0
    };
    let memory_usage = format_memory(get_memory_usage());

    let template = DashboardTemplate {
        online_devices,
        total_devices,
        queued_messages,
        uptime,
        active_connections,
        connection_percent,
        memory_usage,
        pairing_sessions,
        total_teams,
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
        memory_usage_bytes: get_memory_usage(),
    };

    Json(stats).into_response()
}

/// Get current memory usage in bytes (approximate using heap allocations tracking)
fn get_memory_usage() -> u64 {
    // On Unix systems, we can read from /proc/self/statm
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if let Some(rss) = parts.get(1) {
                if let Ok(pages) = rss.parse::<u64>() {
                    // Page size is typically 4KB
                    return pages * 4096;
                }
            }
        }
        0
    }

    // On macOS, use mach APIs via rusage
    #[cfg(target_os = "macos")]
    {
        use std::mem::MaybeUninit;
        let mut rusage = MaybeUninit::uninit();
        // SAFETY: rusage is valid for writing, and RUSAGE_SELF is a valid who parameter
        let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr()) };
        if result == 0 {
            // SAFETY: getrusage succeeded, so rusage is initialized
            let rusage = unsafe { rusage.assume_init() };
            // On macOS, ru_maxrss is in bytes
            return rusage.ru_maxrss as u64;
        }
        0
    }

    // On other platforms, return 0 as we can't easily get memory usage
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        0
    }
}

/// Format memory size to human readable string
fn format_memory(bytes: u64) -> String {
    if bytes == 0 {
        return "N/A".to_string();
    }

    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
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

/// Sessions list page handler
pub async fn sessions_list(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Get pairing sessions
    let sessions = state
        .db
        .list_all_pairing_sessions()
        .await
        .unwrap_or_default();

    let session_views: Vec<SessionView> = sessions
        .into_iter()
        .map(|s| SessionView {
            code: s.code,
            device_name: s.device_name,
            created_at: format_timestamp(s.created_at),
            expires_at: format_timestamp(s.expires_at),
        })
        .collect();

    let template = SessionsTemplate {
        sessions: session_views,
    };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Cancel a pairing session
pub async fn cancel_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(code): Path<String>,
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

    // Cancel the pairing session
    let _ = state.db.cancel_pairing(&code).await;

    Redirect::to("/admin/sessions").into_response()
}

/// Disconnect a device (force offline)
pub async fn disconnect_device(
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

    // Disconnect the device from relay (this closes the WebSocket connection)
    state.relay.unregister(&device_id);

    // Update device status in database
    let _ = state.db.update_device_status(&device_id, false).await;

    tracing::info!("Admin disconnected device: {}", device_id);

    Redirect::to("/admin/devices").into_response()
}

/// Logs page handler
pub async fn logs_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
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

    let level = query.level.unwrap_or_else(|| "all".to_string());
    let entries = LOG_BUFFER.get_entries(Some(&level));

    let log_views: Vec<LogEntryView> = entries
        .into_iter()
        .rev() // Most recent first
        .map(|e| LogEntryView {
            timestamp: e.timestamp,
            level: e.level,
            target: e.target,
            message: e.message,
        })
        .collect();

    let template = LogsTemplate {
        logs: log_views,
        level,
    };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Clear logs handler
pub async fn clear_logs(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    LOG_BUFFER.clear();
    tracing::info!("Admin cleared server logs");

    Redirect::to("/admin/logs").into_response()
}

/// Teams list page handler
pub async fn teams_list(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    // Check admin is enabled
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check authentication
    let cookies = headers.get(COOKIE).and_then(|v| v.to_str().ok());

    if let Some(redirect) = require_auth(cookies, &state.config.session_secret) {
        return redirect.into_response();
    }

    // Get teams
    let teams = state.db.list_all_teams().await.unwrap_or_default();

    let mut team_views = Vec::new();
    for team in teams {
        let member_count = state.db.count_team_members(&team.id).await.unwrap_or(0);
        let pending_invitations = state
            .db
            .count_pending_invitations(&team.id)
            .await
            .unwrap_or(0);

        team_views.push(TeamView {
            id: team.id,
            name: team.name,
            description: team.description,
            member_count,
            pending_invitations,
            broadcast_enabled: team.broadcast_enabled,
            created_at: format_timestamp(team.created_at),
        });
    }

    let template = TeamsTemplate { teams: team_views };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Team details page handler
pub async fn team_details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
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

    // Get team
    let team = match state.db.get_team(&team_id).await {
        Ok(Some(t)) => t,
        _ => return Redirect::to("/admin/teams").into_response(),
    };

    let member_count = state.db.count_team_members(&team.id).await.unwrap_or(0);
    let pending_invitations = state
        .db
        .count_pending_invitations(&team.id)
        .await
        .unwrap_or(0);

    let team_view = TeamView {
        id: team.id.clone(),
        name: team.name,
        description: team.description,
        member_count,
        pending_invitations,
        broadcast_enabled: team.broadcast_enabled,
        created_at: format_timestamp(team.created_at),
    };

    // Get members with device names
    let members = state
        .db
        .get_team_members(&team_id)
        .await
        .unwrap_or_default();
    let mut member_views = Vec::new();
    for member in members {
        let device_name = state
            .db
            .get_device(&member.device_id)
            .await
            .ok()
            .flatten()
            .map(|d| d.device_name)
            .unwrap_or_else(|| "Unknown Device".to_string());

        let is_admin = member.role_enum() == TeamRole::Admin;
        member_views.push(TeamMemberView {
            device_id: member.device_id,
            device_name,
            is_admin,
            joined_at: format_timestamp(member.joined_at),
        });
    }

    // Get invitations
    let invitations = state
        .db
        .get_team_invitations(&team_id)
        .await
        .unwrap_or_default();
    let invitation_views: Vec<TeamInvitationView> = invitations
        .into_iter()
        .map(|inv| TeamInvitationView {
            code: inv.code,
            status: inv.status,
            created_at: format_timestamp(inv.created_at),
            expires_at: format_timestamp(inv.expires_at),
        })
        .collect();

    // Get audit log
    let audit_entries = state
        .db
        .get_team_audit_log(&team_id)
        .await
        .unwrap_or_default();
    let audit_views: Vec<TeamAuditEntryView> = audit_entries
        .into_iter()
        .map(|entry| TeamAuditEntryView {
            action: entry.action,
            device_id: entry.device_id,
            details: entry.details,
            created_at: format_timestamp(entry.created_at),
        })
        .collect();

    let template = TeamDetailsTemplate {
        team: team_view,
        members: member_views,
        invitations: invitation_views,
        audit_entries: audit_views,
    };

    Html(template.render().unwrap_or_default()).into_response()
}

/// Delete a team
pub async fn delete_team(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(team_id): Path<String>,
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

    // Delete the team
    let _ = state.db.delete_team(&team_id).await;
    tracing::info!("Admin deleted team: {}", team_id);

    Redirect::to("/admin/teams").into_response()
}

/// Remove a member from a team
pub async fn remove_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((team_id, device_id)): Path<(String, String)>,
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

    // Remove the member
    let _ = state.db.remove_team_member(&team_id, &device_id).await;

    // Log the action
    let audit_id = uuid::Uuid::new_v4().to_string();
    let _ = state
        .db
        .add_team_audit_entry(
            &audit_id,
            &team_id,
            "admin",
            "member_removed",
            Some(&device_id),
        )
        .await;

    tracing::info!("Admin removed member {} from team {}", device_id, team_id);

    Redirect::to(&format!("/admin/teams/{}", team_id)).into_response()
}
