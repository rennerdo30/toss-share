//! API request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{
    auth::{create_token, verify_signature, AuthenticatedDevice},
    error::{ApiError, ApiResult},
    relay::RelayMessage,
    AppState,
};

// ============================================================================
// Register Device
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub device_id: String,
    pub public_key: String, // Base64 encoded
    pub device_name: String,
    pub timestamp: u64,
    pub signature: String, // Base64 encoded
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub token: String,
    pub expires_at: u64,
}

pub async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<Json<RegisterResponse>> {
    // Decode public key
    let public_key =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &req.public_key)
            .map_err(|_| ApiError::BadRequest("Invalid public key encoding".to_string()))?;

    // Decode signature
    let signature =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &req.signature)
            .map_err(|_| ApiError::BadRequest("Invalid signature encoding".to_string()))?;

    // Verify timestamp freshness (5 minute window)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if now.abs_diff(req.timestamp) > 300 {
        return Err(ApiError::BadRequest("Timestamp too old".to_string()));
    }

    // Verify signature
    let message = format!("register:{}:{}", req.device_id, req.timestamp);
    if !verify_signature(&public_key, message.as_bytes(), &signature)? {
        return Err(ApiError::Unauthorized("Invalid signature".to_string()));
    }

    // Register device in database
    state
        .db
        .upsert_device(&req.device_id, &public_key, &req.device_name)
        .await?;

    // Create JWT token
    let token = create_token(
        &req.device_id,
        &state.config.jwt_secret,
        state.config.jwt_expiration,
    )?;

    let expires_at = now + state.config.jwt_expiration;

    Ok(Json(RegisterResponse { token, expires_at }))
}

// ============================================================================
// Unregister Device
// ============================================================================

pub async fn unregister_device(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
) -> ApiResult<StatusCode> {
    state.db.delete_device(&auth.device_id).await?;
    state.relay.unregister(&auth.device_id);

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Relay Message
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RelayRequest {
    pub encrypted_message: String, // Base64 encoded
}

pub async fn relay_message(
    State(state): State<AppState>,
    auth: AuthenticatedDevice,
    Path(target_device_id): Path<String>,
    Json(req): Json<RelayRequest>,
) -> ApiResult<StatusCode> {
    // Check if target device exists
    let _target = state
        .db
        .get_device(&target_device_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Target device not found".to_string()))?;

    let message = RelayMessage {
        id: Uuid::new_v4().to_string(),
        from_device: auth.device_id.clone(),
        to_device: target_device_id.clone(),
        encrypted_payload: req.encrypted_message.clone(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    // Try to send directly if device is connected
    if state
        .relay
        .send_to(&target_device_id, message.clone())
        .await
    {
        return Ok(StatusCode::ACCEPTED);
    }

    // Otherwise queue for later delivery
    state
        .db
        .queue_message(
            &message.id,
            &auth.device_id,
            &target_device_id,
            &req.encrypted_message,
        )
        .await?;

    Ok(StatusCode::ACCEPTED)
}

// ============================================================================
// Device Status
// ============================================================================

#[derive(Debug, Serialize)]
pub struct DeviceStatusResponse {
    pub device_id: String,
    pub is_online: bool,
    pub last_seen: Option<i64>,
}

pub async fn device_status(
    State(state): State<AppState>,
    _auth: AuthenticatedDevice,
    Path(device_id): Path<String>,
) -> ApiResult<Json<DeviceStatusResponse>> {
    let device = state
        .db
        .get_device(&device_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Device not found".to_string()))?;

    // Check if device is currently connected via WebSocket
    let is_online = state.relay.is_connected(&device_id);

    Ok(Json(DeviceStatusResponse {
        device_id: device.id,
        is_online,
        last_seen: device.last_seen,
    }))
}

// ============================================================================
// Pairing
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterPairingRequest {
    pub code: String,
    pub public_key: String, // Base64 encoded
    pub device_name: String,
    pub expires_in_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RegisterPairingResponse {
    pub code: String,
    pub expires_at: u64,
}

pub async fn register_pairing(
    State(state): State<AppState>,
    Json(req): Json<RegisterPairingRequest>,
) -> ApiResult<Json<RegisterPairingResponse>> {
    // Validate code format (6 digits)
    if req.code.len() != 6 || !req.code.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "Pairing code must be 6 digits".to_string(),
        ));
    }

    // Decode public key
    let public_key =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &req.public_key)
            .map_err(|_| ApiError::BadRequest("Invalid public key encoding".to_string()))?;

    // Validate public key length (32 bytes for X25519)
    if public_key.len() != 32 {
        return Err(ApiError::BadRequest(
            "Public key must be 32 bytes".to_string(),
        ));
    }

    // Calculate expiration (default 5 minutes)
    let expires_in = req.expires_in_secs.unwrap_or(300);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expires_at = now + expires_in;

    // Register pairing session
    state
        .db
        .register_pairing(&req.code, &public_key, &req.device_name, expires_at as i64)
        .await?;

    Ok(Json(RegisterPairingResponse {
        code: req.code,
        expires_at,
    }))
}

#[derive(Debug, Serialize)]
pub struct FindPairingResponse {
    pub code: String,
    pub public_key: String, // Base64 encoded
    pub device_name: String,
    pub expires_at: u64,
}

pub async fn find_pairing(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<Json<FindPairingResponse>> {
    // Validate code format
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "Pairing code must be 6 digits".to_string(),
        ));
    }

    // Find pairing session
    let session = state
        .db
        .find_pairing(&code)
        .await?
        .ok_or_else(|| ApiError::NotFound("Pairing session not found or expired".to_string()))?;

    // Encode public key as base64
    let public_key = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &session.public_key,
    );

    Ok(Json(FindPairingResponse {
        code: session.code,
        public_key,
        device_name: session.device_name,
        expires_at: session.expires_at as u64,
    }))
}

pub async fn cancel_pairing(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<StatusCode> {
    // Cancel pairing session
    let deleted = state.db.cancel_pairing(&code).await?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("Pairing session not found".to_string()))
    }
}
