//! Database models

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Device record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub public_key: Vec<u8>,
    pub device_name: String,
    pub is_online: bool,
    pub last_seen: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Queued message record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub from_device: String,
    pub to_device: String,
    pub encrypted_payload: String,
    pub created_at: i64,
}

/// Pairing session record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PairingSession {
    pub code: String,
    pub public_key: Vec<u8>,
    pub device_name: String,
    pub expires_at: i64,
    pub created_at: i64,
}

/// Team record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub broadcast_enabled: bool,
    pub max_members: i64,
}

/// Team role enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamRole {
    Admin,
    Member,
}

impl TeamRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            TeamRole::Admin => "admin",
            TeamRole::Member => "member",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => TeamRole::Admin,
            _ => TeamRole::Member,
        }
    }
}

/// Team member record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TeamMember {
    pub team_id: String,
    pub device_id: String,
    pub role: String,
    pub joined_at: i64,
}

impl TeamMember {
    pub fn role_enum(&self) -> TeamRole {
        TeamRole::parse(&self.role)
    }
}

/// Team invitation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Revoked,
}

impl InvitationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Revoked => "revoked",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => InvitationStatus::Pending,
            "accepted" => InvitationStatus::Accepted,
            "declined" => InvitationStatus::Declined,
            "revoked" => InvitationStatus::Revoked,
            _ => InvitationStatus::Pending,
        }
    }
}

/// Team invitation record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TeamInvitation {
    pub id: String,
    pub team_id: String,
    pub code: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub status: String,
    pub used_by: Option<String>,
}

impl TeamInvitation {
    pub fn status_enum(&self) -> InvitationStatus {
        InvitationStatus::parse(&self.status)
    }
}

/// Team audit log entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TeamAuditEntry {
    pub id: String,
    pub team_id: String,
    pub device_id: String,
    pub action: String,
    pub details: Option<String>,
    pub created_at: i64,
}
