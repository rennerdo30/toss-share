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
