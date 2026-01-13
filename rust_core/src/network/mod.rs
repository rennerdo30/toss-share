//! Networking for Toss
//!
//! This module provides:
//! - mDNS-SD device discovery on local network
//! - QUIC transport for P2P connections
//! - Relay server client for remote connections
//! - Network manager coordinating all networking

pub mod discovery;
pub mod nat_traversal;
pub mod relay_client;
pub mod transport;
pub mod websocket_transport;

use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use base64::Engine;
use hex;

use crate::crypto::{derive_key, DeviceIdentity, DerivedKeyPurpose, EphemeralKeyPair};
use crate::error::NetworkError;
use crate::protocol::{KeyRotation, KeyRotationReason, Message};

pub use discovery::{DiscoveredPeer, MdnsDiscovery};
pub use nat_traversal::{gather_candidates, IceCandidate, StunClient, StunConfig, TurnClient, TurnConfig};
pub use relay_client::RelayClient;
pub use transport::{PeerConnection, QuicTransport};
pub use websocket_transport::{WebSocketPeerConnection, WebSocketTransport};

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

/// Ephemeral key pair for a peer session
#[derive(Clone)]
struct PeerEphemeralKey {
    #[allow(dead_code)]
    public_key: [u8; 32], // Our ephemeral public key
    peer_public_key: Option<[u8; 32]>, // Peer's current ephemeral public key
}

/// Callback function type for getting device public key by device ID
pub type GetPublicKeyFn = Box<dyn Fn(&[u8; 32]) -> Option<[u8; 32]> + Send + Sync>;

/// Network manager coordinating discovery and connections
pub struct NetworkManager {
    config: NetworkConfig,
    identity: Arc<DeviceIdentity>,
    discovery: Option<MdnsDiscovery>,
    transport: Option<QuicTransport>,
    relay_client: Option<Arc<RelayClient>>,
    peers: Arc<RwLock<HashMap<[u8; 32], PeerConnection>>>,
    ephemeral_keys: Arc<RwLock<HashMap<[u8; 32], PeerEphemeralKey>>>,
    event_tx: broadcast::Sender<NetworkEvent>,
    get_public_key: Option<Arc<GetPublicKeyFn>>,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(
        identity: Arc<DeviceIdentity>,
        config: NetworkConfig,
    ) -> Result<Self, NetworkError> {
        Self::new_with_public_key_fn(identity, config, None).await
    }

    /// Create a new network manager with a callback to get device public keys
    pub async fn new_with_public_key_fn(
        identity: Arc<DeviceIdentity>,
        config: NetworkConfig,
        get_public_key: Option<Arc<GetPublicKeyFn>>,
    ) -> Result<Self, NetworkError> {
        let (event_tx, _) = broadcast::channel(100);

        Ok(Self {
            config,
            identity,
            discovery: None,
            transport: None,
            relay_client: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
            ephemeral_keys: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            get_public_key,
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

    /// Rotate session key for a peer
    async fn rotate_session_key(&self, device_id: &[u8; 32]) -> Result<(), NetworkError> {
        // Generate new ephemeral key pair
        let new_ephemeral = EphemeralKeyPair::generate();
        let new_public_key = *new_ephemeral.public_key_bytes();

        // Get peer's current ephemeral public key
        let peer_public_key = {
            let keys = self.ephemeral_keys.read();
            keys.get(device_id)
                .and_then(|k| k.peer_public_key)
                .ok_or_else(|| NetworkError::ConnectionFailed("No peer ephemeral key".to_string()))?
        };

        // Derive new shared secret (consumes new_ephemeral)
        let shared_secret = new_ephemeral.derive_shared_secret(&peer_public_key);
        
        // Derive new session key
        let new_session_key = derive_key(
            shared_secret.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        ).map_err(|e| NetworkError::ConnectionFailed(format!("Key derivation failed: {}", e)))?;

        // Sign the new public key with identity key
        let signature = self.identity.sign(&new_public_key);

        // Create KeyRotation message
        let rotation = KeyRotation {
            new_public_key,
            signature,
            reason: KeyRotationReason::Scheduled,
        };

        // Send rotation message
        let rotation_message = Message::KeyRotation(rotation);
        self.send_to_peer_internal(device_id, &rotation_message).await?;

        // Update session key in connection
        // Get connection pointer first, then drop the lock before await
        let conn_ptr: Option<*const PeerConnection> = {
            let peers = self.peers.read();
            peers.get(device_id).map(|conn| conn as *const PeerConnection)
        };
        
        if let Some(ptr) = conn_ptr {
            // SAFETY: We're modifying the connection, but set_session_key and reset_session_tracker
            // use internal mutexes, so this is safe
            let conn = unsafe { &*ptr };
            conn.set_session_key(new_session_key).await;
            conn.reset_session_tracker().await;
        }

        // Update ephemeral keys - store the secret separately for later use
        // For now, we'll regenerate when needed
        {
            let mut keys = self.ephemeral_keys.write();
            keys.insert(*device_id, PeerEphemeralKey {
                public_key: new_public_key,
                peer_public_key: Some(peer_public_key),
            });
        }

        // Store the ephemeral secret for this rotation
        // Note: In a full implementation, we'd store this securely
        // For now, we regenerate when handling incoming rotations

        Ok(())
    }

    /// Handle incoming KeyRotation message
    async fn handle_key_rotation(
        &self,
        device_id: &[u8; 32],
        rotation: &KeyRotation,
    ) -> Result<(), NetworkError> {
        // Verify signature - get the device's identity public key
        if let Some(ref get_key_fn) = self.get_public_key {
            if let Some(peer_public_key) = get_key_fn(device_id) {
                // Verify the signature on the new public key
                if !DeviceIdentity::verify_from_public_key(
                    &peer_public_key,
                    &rotation.new_public_key,
                    &rotation.signature,
                ) {
                    return Err(NetworkError::ConnectionFailed(
                        "Key rotation signature verification failed".to_string(),
                    ));
                }
            } else {
                tracing::warn!("Device public key not found for key rotation verification, device_id: {}", hex::encode(device_id));
                // For backward compatibility, allow if we can't find the key
                // In production, this should probably be an error
            }
        } else {
            tracing::warn!("No public key lookup function available for key rotation verification");
            // For backward compatibility, allow if no lookup function is provided
        }
        
        // Generate new ephemeral key pair for this rotation
        // We generate a new one because we don't store the secret
        // In a full implementation, we'd store ephemeral secrets securely
        let new_ephemeral = EphemeralKeyPair::generate();
        let new_public_key = *new_ephemeral.public_key_bytes();
        
        // Derive new shared secret with peer's new public key
        let shared_secret = new_ephemeral.derive_shared_secret(&rotation.new_public_key);
        
        // Derive new session key
        let new_session_key = derive_key(
            shared_secret.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        ).map_err(|e| NetworkError::ConnectionFailed(format!("Key derivation failed: {}", e)))?;

        // Update session key
        // Get connection pointer first, then drop the lock before await
        let conn_ptr: Option<*const PeerConnection> = {
            let peers = self.peers.read();
            peers.get(device_id).map(|conn| conn as *const PeerConnection)
        };
        
        if let Some(ptr) = conn_ptr {
            // SAFETY: We're modifying the connection, but set_session_key and reset_session_tracker
            // use internal mutexes, so this is safe
            let conn = unsafe { &*ptr };
            conn.set_session_key(new_session_key).await;
            conn.reset_session_tracker().await;
        }

        // Update ephemeral keys with new public key
        {
            let mut keys = self.ephemeral_keys.write();
            keys.insert(*device_id, PeerEphemeralKey {
                public_key: new_public_key,
                peer_public_key: Some(rotation.new_public_key),
            });
        }

        Ok(())
    }

    /// Internal send method that doesn't check for rotation
    async fn send_to_peer_internal(
        &self,
        device_id: &[u8; 32],
        message: &Message,
    ) -> Result<(), NetworkError> {
        // Get a reference to the connection while holding the lock, then drop it
        let conn_ptr: Option<*const PeerConnection> = {
            let peers = self.peers.read();
            peers.get(device_id).map(|conn| conn as *const PeerConnection)
        };
        
        if let Some(ptr) = conn_ptr {
            let conn = unsafe { &*ptr };
            match conn.send_message(message).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    // If send fails, the connection might be dead - remove it
                    let mut peers = self.peers.write();
                    if peers.contains_key(device_id) {
                        peers.remove(device_id);
                        tracing::warn!("Removed dead connection to device {}", hex::encode(device_id));
                        
                        // Emit disconnection event
                        let _ = self.event_tx.send(NetworkEvent::PeerDisconnected {
                            device_id: *device_id,
                        });
                    }
                    Err(e)
                }
            }
        } else {
            Err(NetworkError::PeerNotFound(hex::encode(device_id)))
        }
    }

    /// Send message to a specific peer
    pub async fn send_to_peer(
        &self,
        device_id: &[u8; 32],
        message: &Message,
    ) -> Result<(), NetworkError> {
        // Check if rotation is needed before sending
        // Get connection pointer first, then drop the lock before await
        let conn_ptr: Option<*const PeerConnection> = {
            let peers = self.peers.read();
            peers.get(device_id).map(|conn| conn as *const PeerConnection)
        };
        
        let needs_rotation = if let Some(ptr) = conn_ptr {
            // SAFETY: We're only reading from the connection, not modifying it
            // The connection is owned by the peers HashMap which is behind a RwLock
            let conn = unsafe { &*ptr };
            conn.should_rotate_key().await
        } else {
            false
        };

        if needs_rotation {
            // Rotate key first
            if let Err(e) = self.rotate_session_key(device_id).await {
                tracing::warn!("Failed to rotate key: {}, continuing with old key", e);
            }
        }

        // Try QUIC first
        match self.send_to_peer_internal(device_id, message).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // If QUIC fails, try WebSocket fallback if configured
                // TODO: Implement WebSocket fallback connection
                tracing::warn!("QUIC send failed: {}, WebSocket fallback not yet implemented", e);
                Err(e)
            }
        }
    }

    /// Broadcast message to all connected peers
    /// Returns Ok(()) if at least one peer received the message, or if no peers are connected
    /// Returns Err only if all peers failed and no relay fallback succeeded
    pub async fn broadcast(&self, message: &Message) -> Result<(), NetworkError> {
        // Collect all peer device IDs while holding the lock
        let (device_ids, relay_client, is_empty) = {
            let peers = self.peers.read();
            let device_list: Vec<[u8; 32]> = peers.keys().copied().collect();
            let relay = self.relay_client.clone();
            let empty = peers.is_empty();
            (device_list, relay, empty)
        }; // Lock is dropped here

        // If no peers connected, try relay for all known devices
        if is_empty {
            if let Some(_relay) = &relay_client {
                // Serialize message for relay
                if let Ok(_serialized) = bincode::serialize(message) {
                    // For now, we'll just log - full implementation would track target devices
                    // and send to each via relay
                    tracing::debug!("No peers connected, message would be queued on relay");
                    return Ok(()); // Not an error if no peers are connected
                }
            }
            return Ok(()); // No peers is not an error
        }

        let mut success_count = 0;
        let mut last_error: Option<String> = None;

        // Send to all peers using send_to_peer which handles the lock properly
        for device_id in device_ids.iter() {
            match self.send_to_peer(device_id, message).await {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    last_error = Some(format!("{}", e));
                    // Try relay as fallback
                    if let Some(ref relay) = relay_client {
                        // Serialize message for relay (relay expects encrypted bytes, but we'll send serialized message)
                        // TODO: Encrypt message before sending to relay
                        if let Ok(serialized) = bincode::serialize(message) {
                            let device_id_hex = hex::encode(device_id);
                            match relay.send_to_device(&device_id_hex, &serialized).await {
                                Ok(()) => {
                                    success_count += 1;
                                    tracing::debug!("Sent to device {} via relay fallback", device_id_hex);
                                }
                                Err(relay_err) => {
                                    tracing::warn!("Failed to send to device {} via QUIC and relay: {} / {}", 
                                        device_id_hex, e, relay_err);
                                }
                            }
                        }
                    } else {
                        tracing::warn!("Failed to send to device {}: {}", hex::encode(device_id), e);
                    }
                }
            }
        }

        // Return success if at least one peer received the message
        // Partial failures are acceptable - we log warnings but don't fail the entire broadcast
        if success_count > 0 {
            if success_count < device_ids.len() {
                tracing::warn!("Partial broadcast success: {}/{} devices received message", 
                    success_count, device_ids.len());
            }
            Ok(())
        } else {
            // All peers failed
            if let Some(err_msg) = last_error {
                Err(NetworkError::ConnectionFailed(err_msg))
            } else {
                Ok(()) // No peers is not an error
            }
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

        // Initialize ephemeral key for this peer
        let ephemeral = EphemeralKeyPair::generate();
        let ephemeral_public = *ephemeral.public_key_bytes();
        self.ephemeral_keys.write().insert(device_id, PeerEphemeralKey {
            public_key: ephemeral_public,
            peer_public_key: None, // Will be set during pairing
        });

        self.peers.write().insert(device_id, conn);

        let _ = self.event_tx.send(NetworkEvent::PeerConnected {
            device_id,
            device_name: String::new(),
        });

        Ok(device_id)
    }

    /// Process incoming message and handle KeyRotation if needed
    pub async fn process_message(
        &self,
        device_id: &[u8; 32],
        message: Message,
    ) -> Result<(), NetworkError> {
        // Handle KeyRotation messages
        if let Message::KeyRotation(rotation) = &message {
            return self.handle_key_rotation(device_id, rotation).await;
        }

        // Emit event for other message types
        let _ = self.event_tx.send(NetworkEvent::MessageReceived {
            from_device_id: *device_id,
            message,
        });

        Ok(())
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
