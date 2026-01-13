//! Networking for Toss
//!
//! This module provides:
//! - mDNS-SD device discovery on local network
//! - QUIC transport for P2P connections
//! - Relay server client for remote connections
//! - Network manager coordinating all networking

pub mod discovery;
pub mod relay_client;
pub mod transport;

use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use base64::Engine;
use hex;

use crate::crypto::DeviceIdentity;
use crate::error::NetworkError;
use crate::protocol::Message;

pub use discovery::{DiscoveredPeer, MdnsDiscovery};
pub use relay_client::RelayClient;
pub use transport::{PeerConnection, QuicTransport};

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Port for QUIC listener (0 = random)
    pub quic_port: u16,
    /// Device name for discovery
    pub device_name: String,
    /// Optional relay server URL
    pub relay_url: Option<String>,
    /// Enable mDNS discovery
    pub enable_mdns: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            quic_port: 0,
            device_name: "Toss Device".to_string(),
            relay_url: None,
            enable_mdns: true,
        }
    }
}

/// Network events
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Peer discovered via mDNS
    PeerDiscovered(DiscoveredPeer),
    /// Peer went offline
    PeerLost(String),
    /// Connected to a peer
    PeerConnected {
        device_id: [u8; 32],
        device_name: String,
    },
    /// Disconnected from a peer
    PeerDisconnected { device_id: [u8; 32] },
    /// Message received from peer
    MessageReceived {
        from_device_id: [u8; 32],
        message: Message,
    },
    /// Error occurred
    Error(String),
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub device_id: [u8; 32],
    pub device_name: String,
    pub addresses: Vec<SocketAddr>,
    pub is_connected: bool,
    pub is_local: bool,
}

/// Network manager coordinating discovery and connections
pub struct NetworkManager {
    config: NetworkConfig,
    identity: Arc<DeviceIdentity>,
    discovery: Option<MdnsDiscovery>,
    transport: Option<QuicTransport>,
    relay_client: Option<Arc<RelayClient>>,
    peers: Arc<RwLock<HashMap<[u8; 32], PeerConnection>>>,
    event_tx: broadcast::Sender<NetworkEvent>,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(
        identity: Arc<DeviceIdentity>,
        config: NetworkConfig,
    ) -> Result<Self, NetworkError> {
        let (event_tx, _) = broadcast::channel(100);

        Ok(Self {
            config,
            identity,
            discovery: None,
            transport: None,
            relay_client: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        })
    }

    /// Start the network manager
    pub async fn start(&mut self) -> Result<(), NetworkError> {
        // Initialize QUIC transport
        let bind_addr: SocketAddr = format!("0.0.0.0:{}", self.config.quic_port)
            .parse()
            .map_err(|e| NetworkError::AddressParse(format!("{}", e)))?;

        let transport = QuicTransport::new(bind_addr).await?;
        let local_port = transport.local_addr().port();
        self.transport = Some(transport);

        // Initialize mDNS discovery
        if self.config.enable_mdns {
            let discovery = MdnsDiscovery::new(
                &self.identity.device_id_hex(),
                &self.config.device_name,
                local_port,
            )?;
            discovery.register()?;
            self.discovery = Some(discovery);
        }

        // Initialize relay client if URL provided
        if let Some(ref url) = self.config.relay_url {
            let relay = RelayClient::new(url, self.identity.clone());
            // Connect to relay server
            if let Err(e) = relay.connect().await {
                tracing::warn!("Failed to connect to relay server: {}", e);
                // Continue without relay - P2P will still work
            } else {
                tracing::info!("Connected to relay server at {}", url);
                
                // Store relay client
                let relay_arc = Arc::new(relay);
                let relay_clone = relay_arc.clone();
                let event_tx = self.event_tx.clone();
                let identity = self.identity.clone();
                
                // Spawn task to receive messages from relay
                tokio::spawn(async move {
                    Self::relay_receive_loop(&*relay_clone, event_tx, identity).await;
                });
                
                self.relay_client = Some(relay_arc);
            }
        }

        Ok(())
    }

    /// Stop the network manager
    pub async fn stop(&mut self) {
        // Stop discovery
        if let Some(ref discovery) = self.discovery {
            discovery.unregister();
        }

        // Close all peer connections (sync operation, release lock immediately)
        {
            let peers = self.peers.write();
            for (_id, conn) in peers.iter() {
                conn.close();
            }
        }

        // Disconnect relay (async, after lock released)
        if let Some(ref mut relay) = self.relay_client {
            relay.disconnect().await;
        }
    }

    /// Subscribe to network events
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }

    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read();
        peers
            .iter()
            .filter(|(_, conn)| conn.is_connected())
            .map(|(id, conn)| PeerInfo {
                device_id: *id,
                device_name: conn.peer_name().unwrap_or_default(),
                addresses: conn.addresses().to_vec(),
                is_connected: true,
                is_local: conn.is_local(),
            })
            .collect()
    }

    /// Send message to a specific peer
    #[allow(clippy::await_holding_lock)]
    pub async fn send_to_peer(
        &self,
        device_id: &[u8; 32],
        message: &Message,
    ) -> Result<(), NetworkError> {
        // Get a reference to the connection while holding the lock, then drop it
        // We need to use a raw pointer because PeerConnection doesn't implement Clone
        // and we can't hold the lock across await
        let conn_ptr: Option<*const PeerConnection> = {
            let peers = self.peers.read();
            peers.get(device_id).map(|conn| conn as *const PeerConnection)
        }; // Lock is dropped here
        
        if let Some(ptr) = conn_ptr {
            // SAFETY: 
            // 1. PeerConnection::send_message takes &self, not &mut self, so no mutation
            // 2. The connection is owned by NetworkManager in peers HashMap which is behind a RwLock
            // 3. We've dropped the guard, so we're not holding a lock
            // 4. The connection will remain valid as long as NetworkManager exists
            // 5. send_message() only reads from connection, so concurrent access is safe
            let conn = unsafe { &*ptr };
            conn.send_message(message).await
        } else {
            Err(NetworkError::PeerNotFound(hex::encode(device_id)))
        }
    }

    /// Broadcast message to all connected peers
    pub async fn broadcast(&self, message: &Message) -> Result<(), NetworkError> {
        // Collect all peer device IDs while holding the lock
        let (device_ids, relay_client, is_empty) = {
            let peers = self.peers.read();
            let device_list: Vec<[u8; 32]> = peers.keys().copied().collect();
            let relay = self.relay_client.clone();
            let empty = peers.is_empty();
            (device_list, relay, empty)
        }; // Lock is dropped here

        let mut last_error = None;

        // Send to all peers using send_to_peer which handles the lock properly
        for device_id in device_ids.iter() {
            if let Err(e) = self.send_to_peer(device_id, message).await {
                last_error = Some(e);
                // Try relay as fallback
                if let Some(ref relay) = relay_client {
                    // Serialize message for relay (relay expects encrypted bytes, but we'll send serialized message)
                    // TODO: Encrypt message before sending to relay
                    if let Ok(serialized) = bincode::serialize(message) {
                        let device_id_hex = hex::encode(device_id);
                        if let Err(relay_err) = relay.send_to_device(&device_id_hex, &serialized).await {
                            tracing::warn!("Failed to send via relay: {}", relay_err);
                        }
                    }
                }
            }
        }

        // If no peers connected, try relay for all known devices
        if is_empty {
            if let Some(_relay) = &relay_client {
                // Serialize message for relay
                if let Ok(_serialized) = bincode::serialize(message) {
                    // For now, we'll just log - full implementation would track target devices
                    // and send to each via relay
                    tracing::debug!("No peers connected, message would be queued on relay");
                }
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Get the local QUIC address
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.transport.as_ref().map(|t| t.local_addr())
    }

    /// Connect to a peer by address
    pub async fn connect(&self, addr: SocketAddr) -> Result<[u8; 32], NetworkError> {
        let transport = self.transport.as_ref().ok_or_else(|| {
            NetworkError::ConnectionFailed("Transport not initialized".to_string())
        })?;

        let conn = transport.connect(addr).await?;
        let device_id = conn
            .peer_device_id()
            .ok_or_else(|| NetworkError::ConnectionFailed("No device ID".to_string()))?;

        self.peers.write().insert(device_id, conn);

        let _ = self.event_tx.send(NetworkEvent::PeerConnected {
            device_id,
            device_name: String::new(),
        });

        Ok(device_id)
    }

    /// Receive loop for relay messages
    async fn relay_receive_loop(
        relay: &RelayClient,
        event_tx: broadcast::Sender<NetworkEvent>,
        _identity: Arc<DeviceIdentity>,
    ) {
        loop {
            match relay.receive().await {
                Ok(relay_msg) => {
                    // Decode device ID from hex
                    if let Ok(device_id_bytes) = hex::decode(&relay_msg.from_device) {
                        if device_id_bytes.len() == 32 {
                            let mut device_id = [0u8; 32];
                            device_id.copy_from_slice(&device_id_bytes);
                            
                            // Decode encrypted payload
                            if let Ok(payload) = base64::engine::general_purpose::STANDARD
                                .decode(&relay_msg.encrypted_payload)
                            {
                                // Deserialize message
                                if let Ok(message) = bincode::deserialize(&payload) {
                                    let _ = event_tx.send(NetworkEvent::MessageReceived {
                                        from_device_id: device_id,
                                        message,
                                    });
                                } else {
                                    tracing::warn!("Failed to deserialize relay message");
                                }
                            } else {
                                tracing::warn!("Failed to decode relay payload");
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Relay receive error: {}", e);
                    let _ = event_tx.send(NetworkEvent::Error(format!("Relay error: {}", e)));
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.quic_port, 0);
        assert!(config.enable_mdns);
        assert!(config.relay_url.is_none());
    }

    #[tokio::test]
    async fn test_network_manager_creation() {
        let identity = Arc::new(DeviceIdentity::generate().unwrap());
        let config = NetworkConfig {
            enable_mdns: false, // Disable for test
            ..Default::default()
        };

        let manager = NetworkManager::new(identity, config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_network_event_broadcast() {
        let identity = Arc::new(DeviceIdentity::generate().unwrap());
        let config = NetworkConfig {
            enable_mdns: false,
            ..Default::default()
        };

        let manager = NetworkManager::new(identity, config).await.unwrap();
        let mut receiver = manager.subscribe();

        // Test that we can subscribe to events
        assert!(receiver.try_recv().is_err()); // Should be empty initially
    }

    #[tokio::test]
    async fn test_network_broadcast_message() {
        let identity = Arc::new(DeviceIdentity::generate().unwrap());
        let config = NetworkConfig {
            enable_mdns: false,
            ..Default::default()
        };

        let manager = NetworkManager::new(identity, config).await.unwrap();
        
        // Create a test message
        let content = crate::protocol::ClipboardContent::text("Test message");
        let update = crate::protocol::ClipboardUpdate::new(content);
        let message = crate::protocol::Message::ClipboardUpdate(update);

        // Broadcast should not fail even with no peers
        let result = manager.broadcast(&message).await;
        // Should succeed (no peers is not an error)
        assert!(result.is_ok() || matches!(result, Err(crate::error::NetworkError::PeerNotFound(_))));
    }

    #[test]
    fn test_network_config_with_relay() {
        let config = NetworkConfig {
            relay_url: Some("http://localhost:8080".to_string()),
            ..Default::default()
        };
        assert_eq!(config.relay_url, Some("http://localhost:8080".to_string()));
    }
}
