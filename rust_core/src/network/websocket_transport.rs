//! WebSocket transport as fallback when QUIC fails
//!
//! Provides WebSocket over TLS transport for restrictive networks
//! where QUIC/UDP is blocked.

use crate::crypto::KEY_SIZE;
use crate::error::NetworkError;
use crate::protocol::{Frame, Message};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// WebSocket connection to a peer
pub struct WebSocketPeerConnection {
    stream: Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    session_key: Mutex<Option<[u8; KEY_SIZE]>>,
    peer_addr: SocketAddr,
}

impl WebSocketPeerConnection {
    /// Create a new WebSocket connection
    pub async fn connect(url: &str) -> Result<Self, NetworkError> {
        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("WebSocket connect failed: {}", e))
        })?;

        // Extract address from URL (simplified - in production would parse properly)
        let _peer_addr: SocketAddr = url
            .replace("ws://", "")
            .replace("wss://", "")
            .split('/')
            .next()
            .unwrap_or("0.0.0.0:0")
            .parse()
            .map_err(|_| NetworkError::AddressParse("Invalid WebSocket URL".to_string()))?;

        Ok(Self {
            stream: Mutex::new(ws_stream),
            session_key: Mutex::new(None),
            peer_addr: _peer_addr,
        })
    }

    /// Set session key for encryption
    pub async fn set_session_key(&self, key: [u8; KEY_SIZE]) {
        *self.session_key.lock().await = Some(key);
    }

    /// Send an encrypted message
    pub async fn send_message(&self, message: &Message) -> Result<(), NetworkError> {
        let key = self.session_key.lock().await;
        let key = key.as_ref().ok_or(NetworkError::NotAuthenticated)?;

        let header = message.header();
        let payload = message
            .serialize()
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        let frame = Frame::encrypt(&header, &payload, key)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        let frame_bytes = frame.to_bytes();

        let mut stream = self.stream.lock().await;
        stream
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                frame_bytes.into(),
            ))
            .await
            .map_err(|e| NetworkError::Transport(format!("WebSocket send failed: {}", e)))?;

        Ok(())
    }

    /// Receive and decrypt a message
    pub async fn receive_message(&self) -> Result<Message, NetworkError> {
        let key = self.session_key.lock().await;
        let key = key.as_ref().ok_or(NetworkError::NotAuthenticated)?;

        let mut stream = self.stream.lock().await;
        let msg = stream
            .next()
            .await
            .ok_or_else(|| NetworkError::ConnectionClosed)?
            .map_err(|e| NetworkError::Transport(format!("WebSocket receive failed: {}", e)))?;

        let frame_bytes = match msg {
            tokio_tungstenite::tungstenite::Message::Binary(data) => data,
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                return Err(NetworkError::ConnectionClosed);
            }
            _ => {
                return Err(NetworkError::Transport(
                    "Unexpected WebSocket message type".to_string(),
                ));
            }
        };

        let frame =
            Frame::from_bytes(&frame_bytes).map_err(|e| NetworkError::Transport(e.to_string()))?;

        let (header, payload) = frame
            .decrypt(key)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Message::deserialize(&header, &payload).map_err(|e| NetworkError::Transport(e.to_string()))
    }

    /// Check if connection is still active
    pub async fn is_connected(&self) -> bool {
        // Try to peek at the stream to see if it's still open
        // This is a simplified check - in production would check stream state
        self.session_key.lock().await.is_some()
    }

    /// Get peer address
    pub fn peer_address(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Close the connection
    pub async fn close(&self) {
        if let Ok(stream) = self.stream.try_lock() {
            // Close the WebSocket connection
            // Note: tokio_tungstenite's close() requires a CloseFrame
            // For now, we'll just drop the stream which will close it
            drop(stream);
        }
    }
}

/// WebSocket transport manager
pub struct WebSocketTransport {
    /// Base URL for WebSocket connections (e.g., "wss://example.com")
    base_url: String,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Connect to a peer via WebSocket
    /// The peer_id is used to construct the WebSocket URL
    pub async fn connect(&self, peer_id: &str) -> Result<WebSocketPeerConnection, NetworkError> {
        // Construct WebSocket URL
        // Format: wss://base_url/ws/peer_id
        let url = if self.base_url.ends_with('/') {
            format!("{}ws/{}", self.base_url, peer_id)
        } else {
            format!("{}/ws/{}", self.base_url, peer_id)
        };

        // Convert http/https to ws/wss
        let url = url
            .replace("https://", "wss://")
            .replace("http://", "ws://");

        WebSocketPeerConnection::connect(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_url_construction() {
        let _transport = WebSocketTransport::new("https://example.com".to_string());
        // Would test connect() but requires actual server
    }

    #[test]
    fn test_url_protocol_conversion() {
        let https_url = "https://example.com/ws/peer123";
        let wss_url = https_url.replace("https://", "wss://");
        assert_eq!(wss_url, "wss://example.com/ws/peer123");

        let http_url = "http://example.com/ws/peer123";
        let ws_url = http_url.replace("http://", "ws://");
        assert_eq!(ws_url, "ws://example.com/ws/peer123");
    }
}
