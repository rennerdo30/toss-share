//! Real-time relay functionality

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Message to be relayed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessage {
    pub id: String,
    pub from_device: String,
    pub to_device: String,
    pub encrypted_payload: String,
    pub timestamp: u64,
}

/// Relay state managing active connections
pub struct RelayState {
    /// Active WebSocket connections: device_id -> message sender
    connections: DashMap<String, mpsc::Sender<RelayMessage>>,
}

impl RelayState {
    /// Create new relay state
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    /// Register a new connection
    pub fn register(&self, device_id: String, sender: mpsc::Sender<RelayMessage>) {
        self.connections.insert(device_id, sender);
    }

    /// Unregister a connection
    pub fn unregister(&self, device_id: &str) {
        self.connections.remove(device_id);
    }

    /// Check if a device is connected
    pub fn is_connected(&self, device_id: &str) -> bool {
        self.connections.contains_key(device_id)
    }

    /// Send a message to a connected device
    pub async fn send_to(&self, device_id: &str, message: RelayMessage) -> bool {
        if let Some(sender) = self.connections.get(device_id) {
            sender.send(message).await.is_ok()
        } else {
            false
        }
    }

    /// Get count of connected devices
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for RelayState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_relay_state() {
        let state = RelayState::new();
        let (tx, mut rx) = mpsc::channel(10);

        state.register("device1".to_string(), tx);
        assert!(state.is_connected("device1"));
        assert!(!state.is_connected("device2"));
        assert_eq!(state.connection_count(), 1);

        let msg = RelayMessage {
            id: "msg1".to_string(),
            from_device: "device2".to_string(),
            to_device: "device1".to_string(),
            encrypted_payload: "test".to_string(),
            timestamp: 0,
        };

        assert!(state.send_to("device1", msg.clone()).await);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, "msg1");

        state.unregister("device1");
        assert!(!state.is_connected("device1"));
    }
}
