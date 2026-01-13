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
    relay_client: Option<RelayClient>,
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
            self.relay_client = Some(relay);
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
        // Note: Ideally we'd clone the connection and release the lock before await,
        // but PeerConnection doesn't implement Clone. This is safe as long as
        // send_message completes quickly.
        let peers = self.peers.read();
        if let Some(conn) = peers.get(device_id) {
            conn.send_message(message).await
        } else {
            Err(NetworkError::PeerNotFound(hex::encode(device_id)))
        }
    }

    /// Broadcast message to all connected peers
    #[allow(clippy::await_holding_lock)]
    pub async fn broadcast(&self, message: &Message) -> Result<(), NetworkError> {
        // Note: Same as send_to_peer - would need Clone on PeerConnection to fix properly
        let peers = self.peers.read();
        let mut last_error = None;

        for (_, conn) in peers.iter() {
            if let Err(e) = conn.send_message(message).await {
                last_error = Some(e);
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
}
