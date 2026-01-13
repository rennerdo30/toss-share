//! Relay server client for remote clipboard sync

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::crypto::DeviceIdentity;
use crate::error::NetworkError;

/// Relay client for connecting to remote relay server
pub struct RelayClient {
    url: String,
    identity: Arc<DeviceIdentity>,
    ws: Mutex<Option<WebSocketConnection>>,
    auth_token: Mutex<Option<String>>,
}

type WebSocketConnection =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Registration request
#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct RegisterRequest {
    device_id: String,
    public_key: String,
    device_name: String,
    timestamp: u64,
    signature: String,
}

/// Registration response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RegisterResponse {
    token: String,
    expires_at: u64,
}

/// Relay message wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct RelayMessage {
    pub from_device: String,
    pub to_device: String,
    pub encrypted_payload: String,
    pub timestamp: u64,
}

impl RelayClient {
    /// Create a new relay client
    pub fn new(url: &str, identity: Arc<DeviceIdentity>) -> Self {
        Self {
            url: url.to_string(),
            identity,
            ws: Mutex::new(None),
            auth_token: Mutex::new(None),
        }
    }

    /// Connect to the relay server
    pub async fn connect(&self) -> Result<(), NetworkError> {
        let ws_url = format!("{}/api/v1/ws", self.url.replace("http", "ws"));

        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| NetworkError::Relay(format!("WebSocket connection failed: {}", e)))?;

        *self.ws.lock().await = Some(ws_stream);

        // Authenticate
        self.authenticate().await?;

        Ok(())
    }

    /// Authenticate with the relay server
    async fn authenticate(&self) -> Result<(), NetworkError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let device_id = self.identity.device_id_hex();
        let challenge = format!("auth:{}:{}", device_id, timestamp);
        let signature = self.identity.sign(challenge.as_bytes());

        let auth_msg = serde_json::json!({
            "type": "auth",
            "device_id": device_id,
            "timestamp": timestamp,
            "signature": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature),
        });

        self.send_ws_message(&auth_msg.to_string()).await?;

        // Wait for auth response
        let response = self.receive_ws_message().await?;
        let auth_response: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| NetworkError::Relay(format!("Invalid auth response: {}", e)))?;

        if auth_response.get("success").and_then(|v| v.as_bool()) == Some(true) {
            if let Some(token) = auth_response.get("token").and_then(|v| v.as_str()) {
                *self.auth_token.lock().await = Some(token.to_string());
            }
            Ok(())
        } else {
            let error = auth_response
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(NetworkError::Relay(format!(
                "Authentication failed: {}",
                error
            )))
        }
    }

    /// Send a message to another device via relay
    pub async fn send_to_device(
        &self,
        target_device_id: &str,
        encrypted_payload: &[u8],
    ) -> Result<(), NetworkError> {
        let msg = RelayMessage {
            from_device: self.identity.device_id_hex(),
            to_device: target_device_id.to_string(),
            encrypted_payload: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                encrypted_payload,
            ),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        let json = serde_json::to_string(&serde_json::json!({
            "type": "relay",
            "message": msg,
        }))
        .map_err(|e| NetworkError::Relay(e.to_string()))?;

        self.send_ws_message(&json).await
    }

    /// Receive a message from the relay
    pub async fn receive(&self) -> Result<RelayMessage, NetworkError> {
        let response = self.receive_ws_message().await?;

        let envelope: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| NetworkError::Relay(format!("Invalid message: {}", e)))?;

        if envelope.get("type").and_then(|v| v.as_str()) == Some("relay") {
            let msg: RelayMessage =
                serde_json::from_value(envelope.get("message").cloned().unwrap_or_default())
                    .map_err(|e| NetworkError::Relay(format!("Invalid relay message: {}", e)))?;
            Ok(msg)
        } else {
            Err(NetworkError::Relay("Unexpected message type".to_string()))
        }
    }

    /// Send WebSocket message
    async fn send_ws_message(&self, message: &str) -> Result<(), NetworkError> {
        let mut ws = self.ws.lock().await;
        let ws = ws
            .as_mut()
            .ok_or_else(|| NetworkError::Relay("Not connected".to_string()))?;

        ws.send(WsMessage::Text(message.to_string().into()))
            .await
            .map_err(|e| NetworkError::Relay(format!("Send failed: {}", e)))
    }

    /// Receive WebSocket message
    async fn receive_ws_message(&self) -> Result<String, NetworkError> {
        let mut ws = self.ws.lock().await;
        let ws = ws
            .as_mut()
            .ok_or_else(|| NetworkError::Relay("Not connected".to_string()))?;

        loop {
            match ws.next().await {
                Some(Ok(WsMessage::Text(text))) => return Ok(text.to_string()),
                Some(Ok(WsMessage::Ping(data))) => {
                    ws.send(WsMessage::Pong(data)).await.ok();
                }
                Some(Ok(WsMessage::Close(_))) => {
                    return Err(NetworkError::ConnectionClosed);
                }
                Some(Err(e)) => {
                    return Err(NetworkError::Relay(format!("Receive error: {}", e)));
                }
                None => {
                    return Err(NetworkError::ConnectionClosed);
                }
                _ => continue,
            }
        }
    }

    /// Disconnect from the relay server
    pub async fn disconnect(&self) {
        if let Some(mut ws) = self.ws.lock().await.take() {
            let _ = ws.close(None).await;
        }
        *self.auth_token.lock().await = None;
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.ws.lock().await.is_some()
    }

    /// Get the relay URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_client_creation() {
        let identity = Arc::new(DeviceIdentity::generate().unwrap());
        let client = RelayClient::new("http://localhost:8080", identity);
        assert_eq!(client.url(), "http://localhost:8080");
    }

    #[test]
    fn test_relay_message_serialization() {
        let msg = RelayMessage {
            from_device: "device1".to_string(),
            to_device: "device2".to_string(),
            encrypted_payload: "dGVzdA==".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("device1"));
        assert!(json.contains("device2"));
    }
}
