//! WebSocket handling for real-time relay

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{auth::verify_signature, relay::RelayMessage, AppState};

/// WebSocket authentication message (for documentation)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct WsAuthMessage {
    device_id: String,
    timestamp: u64,
    signature: String,
}

/// WebSocket relay message (for documentation)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct WsRelayRequest {
    to_device: String,
    encrypted_payload: String,
}

/// WebSocket message envelope
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "auth")]
    Auth {
        device_id: String,
        timestamp: u64,
        signature: String,
    },
    #[serde(rename = "relay")]
    Relay { message: RelayMessage },
    #[serde(rename = "send")]
    Send {
        to_device: String,
        encrypted_payload: String,
    },
    #[serde(rename = "auth_response")]
    AuthResponse { success: bool, error: Option<String> },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Handle WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for authentication message
    let device_id = match authenticate(&mut receiver, &state).await {
        Ok(id) => {
            // Send success response
            let response = WsMessage::AuthResponse {
                success: true,
                error: None,
            };
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
            id
        }
        Err(e) => {
            // Send error response
            let response = WsMessage::AuthResponse {
                success: false,
                error: Some(e),
            };
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
            return;
        }
    };

    tracing::info!("Device {} connected via WebSocket", device_id);

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::channel::<RelayMessage>(100);

    // Register connection
    state.relay.register(device_id.clone(), tx);

    // Update device status
    let _ = state.db.update_device_status(&device_id, true).await;

    // Deliver queued messages
    if let Ok(queued) = state.db.get_queued_messages(&device_id).await {
        for msg in queued {
            let relay_msg = RelayMessage {
                id: msg.id,
                from_device: msg.from_device,
                to_device: msg.to_device,
                encrypted_payload: msg.encrypted_payload,
                timestamp: msg.created_at as u64,
            };
            let envelope = WsMessage::Relay { message: relay_msg };
            if let Ok(json) = serde_json::to_string(&envelope) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
        // Clear delivered messages
        let _ = state.db.delete_queued_messages(&device_id).await;
    }

    // Main loop
    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = handle_client_message(&text.to_string(), &device_id, &state).await {
                            let error = WsMessage::Error { message: e };
                            if let Ok(json) = serde_json::to_string(&error) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }

            // Handle outgoing relay messages
            Some(relay_msg) = rx.recv() => {
                let envelope = WsMessage::Relay { message: relay_msg };
                if let Ok(json) = serde_json::to_string(&envelope) {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }

            else => break,
        }
    }

    // Cleanup
    tracing::info!("Device {} disconnected", device_id);
    state.relay.unregister(&device_id);
    let _ = state.db.update_device_status(&device_id, false).await;
}

/// Authenticate WebSocket connection
async fn authenticate(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    state: &AppState,
) -> Result<String, String> {
    // Wait for auth message with timeout
    let auth_msg = tokio::time::timeout(std::time::Duration::from_secs(10), receiver.next())
        .await
        .map_err(|_| "Authentication timeout".to_string())?
        .ok_or("Connection closed".to_string())?
        .map_err(|e| format!("WebSocket error: {}", e))?;

    let text = match auth_msg {
        Message::Text(t) => t.to_string(),
        _ => return Err("Expected text message".to_string()),
    };

    let msg: WsMessage = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid message format: {}", e))?;

    let (device_id, timestamp, signature) = match msg {
        WsMessage::Auth {
            device_id,
            timestamp,
            signature,
        } => (device_id, timestamp, signature),
        _ => return Err("Expected auth message".to_string()),
    };

    // Verify timestamp freshness
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if now.abs_diff(timestamp) > 300 {
        return Err("Timestamp too old".to_string());
    }

    // Get device from database
    let device = state
        .db
        .get_device(&device_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("Device not registered".to_string())?;

    // Verify signature
    let sig_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &signature)
        .map_err(|_| "Invalid signature encoding".to_string())?;

    let message = format!("auth:{}:{}", device_id, timestamp);
    let valid = verify_signature(&device.public_key, message.as_bytes(), &sig_bytes)
        .map_err(|e| format!("Signature verification error: {}", e))?;

    if !valid {
        return Err("Invalid signature".to_string());
    }

    Ok(device_id)
}

/// Handle incoming client message
async fn handle_client_message(
    text: &str,
    from_device: &str,
    state: &AppState,
) -> Result<(), String> {
    let msg: WsMessage = serde_json::from_str(text)
        .map_err(|e| format!("Invalid message: {}", e))?;

    match msg {
        WsMessage::Send {
            to_device,
            encrypted_payload,
        } => {
            let relay_msg = RelayMessage {
                id: Uuid::new_v4().to_string(),
                from_device: from_device.to_string(),
                to_device: to_device.clone(),
                encrypted_payload: encrypted_payload.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            };

            // Try direct delivery
            if !state.relay.send_to(&to_device, relay_msg.clone()).await {
                // Queue for later
                state
                    .db
                    .queue_message(
                        &relay_msg.id,
                        from_device,
                        &to_device,
                        &encrypted_payload,
                    )
                    .await
                    .map_err(|e| format!("Failed to queue message: {}", e))?;
            }

            Ok(())
        }
        _ => Err("Unexpected message type".to_string()),
    }
}
