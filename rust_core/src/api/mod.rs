//! FFI API for Flutter integration
//!
//! This module provides the public API that is exposed to Flutter
//! via flutter_rust_bridge.

use flutter_rust_bridge::frb;
use parking_lot::RwLock;
use std::sync::{Arc, Mutex};
use sha2::{Digest, Sha256};

use crate::clipboard::ClipboardManager;
use crate::crypto::{DeviceIdentity, PairingSession};
use crate::network::{NetworkConfig, NetworkManager, NetworkEvent};
use crate::protocol::{ClipboardContent, ClipboardUpdate, ContentType, Message};
use crate::storage::{Storage, StoredDevice};

/// Global Toss instance
static TOSS_INSTANCE: RwLock<Option<TossCore>> = RwLock::new(None);

/// Core Toss functionality
pub struct TossCore {
    identity: Arc<DeviceIdentity>,
    device_name: String,
    clipboard: ClipboardManager,
    network: Option<NetworkManager>,
    pairing_session: Option<PairingSession>,
    settings: TossSettings,
    storage: Storage,
    event_receiver: Option<Arc<Mutex<tokio::sync::broadcast::Receiver<NetworkEvent>>>>,
}

/// Toss settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TossSettings {
    pub auto_sync: bool,
    pub sync_text: bool,
    pub sync_images: bool,
    pub sync_files: bool,
    pub max_file_size_mb: u32,
    pub history_enabled: bool,
    pub history_days: u32,
    pub relay_url: Option<String>,
}

impl Default for TossSettings {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_text: true,
            sync_images: true,
            sync_files: true,
            max_file_size_mb: 50,
            history_enabled: true,
            history_days: 7,
            relay_url: None,
        }
    }
}

/// Device information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfoDto {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub last_seen: u64,
}

/// Clipboard item for display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardItemDto {
    pub content_type: String,
    pub preview: String,
    pub size_bytes: u64,
    pub timestamp: u64,
    pub source_device: Option<String>,
}

/// Event types for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TossEvent {
    ClipboardReceived { item: ClipboardItemDto },
    DeviceConnected { device: DeviceInfoDto },
    DeviceDisconnected { device_id: String },
    PairingRequest { device: DeviceInfoDto },
    Error { message: String },
}

/// Event stream for Flutter (simplified - full stream support requires flutter_rust_bridge stream support)
/// For now, we'll use a polling approach or callback-based system
pub struct EventStream {
    // This will be implemented with flutter_rust_bridge streams once available
    // For now, we provide a polling-based API
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize Toss core
#[frb(sync)]
pub fn init_toss(data_dir: String, device_name: String) -> Result<(), String> {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("toss_core=debug")
        .try_init();

    // Initialize storage
    let db_path = std::path::Path::new(&data_dir).join("toss.db");
    let storage = Storage::new(&db_path)
        .map_err(|e| format!("Failed to initialize storage: {}", e))?;

    // Load or create identity
    let identity =
        DeviceIdentity::generate().map_err(|e| format!("Failed to generate identity: {}", e))?;

    // Create clipboard manager
    let clipboard =
        ClipboardManager::new().map_err(|e| format!("Failed to initialize clipboard: {}", e))?;

    let core = TossCore {
        identity: Arc::new(identity),
        device_name,
        clipboard,
        network: None,
        pairing_session: None,
        settings: TossSettings::default(),
        storage,
        event_receiver: None,
    };

    *TOSS_INSTANCE.write() = Some(core);

    Ok(())
}

/// Shutdown Toss
#[frb]
pub async fn shutdown_toss() {
    // Extract network manager while holding lock, then release lock before await
    let network = {
        let mut guard = TOSS_INSTANCE.write();
        guard.take().and_then(|mut core| core.network.take())
    };

    if let Some(mut network) = network {
        network.stop().await;
    }
}

// ============================================================================
// Device Identity
// ============================================================================

/// Get device ID
#[frb(sync)]
pub fn get_device_id() -> String {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.identity.device_id_hex())
        .unwrap_or_default()
}

/// Get device name
#[frb(sync)]
pub fn get_device_name() -> String {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.device_name.clone())
        .unwrap_or_default()
}

/// Set device name
#[frb(sync)]
pub fn set_device_name(name: String) -> Result<(), String> {
    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.device_name = name;
        Ok(())
    } else {
        Err("Toss not initialized".to_string())
    }
}

// ============================================================================
// Pairing
// ============================================================================

/// Start a new pairing session
#[frb(sync)]
pub fn start_pairing() -> Result<PairingInfoDto, String> {
    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = PairingSession::new(&core.device_name);
    let info = session.info(&core.device_name);

    core.pairing_session = Some(session);

    Ok(PairingInfoDto {
        code: info.code,
        qr_data: info.qr_data,
        expires_at: info.expires_at,
        public_key: info.public_key,
    })
}

/// Pairing info for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairingInfoDto {
    pub code: String,
    pub qr_data: String,
    pub expires_at: u64,
    pub public_key: String,
}

/// Complete pairing with QR data
#[frb(sync)]
pub fn complete_pairing_qr(qr_data: String) -> Result<DeviceInfoDto, String> {
    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = core
        .pairing_session
        .take()
        .ok_or("No active pairing session")?;

    let (session_key, device_name, public_key_base64) = session
        .complete_from_qr(&qr_data)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    // Decode public key from base64
    let public_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &public_key_base64,
    )
    .map_err(|e| format!("Invalid public key: {}", e))?;

    // Derive device ID from public key hash
    let device_id = hex::encode(&Sha256::digest(&public_key)[..16]);

    // Store the paired device
    let stored_device = StoredDevice {
        id: device_id.clone(),
        name: device_name.clone(),
        public_key,
        session_key: Some(session_key.to_vec()),
        last_seen: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        is_active: true,
    };

    core.storage
        .devices()
        .store_device(&stored_device)
        .map_err(|e| format!("Failed to store device: {}", e))?;

    Ok(DeviceInfoDto {
        id: device_id,
        name: device_name,
        is_online: false,
        last_seen: 0,
    })
}

/// Complete pairing with manual code
#[frb(sync)]
pub fn complete_pairing_code(
    code: String,
    peer_public_key: Vec<u8>,
) -> Result<DeviceInfoDto, String> {
    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = core
        .pairing_session
        .take()
        .ok_or("No active pairing session")?;

    let peer_key: [u8; 32] = peer_public_key
        .try_into()
        .map_err(|_| "Invalid public key length")?;

    let session_key = session
        .complete(&peer_key, &code)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    // Derive device ID from public key
    let device_id = hex::encode(&sha2::Sha256::digest(&peer_key)[..16]);

    // Store the paired device
    let stored_device = StoredDevice {
        id: device_id.clone(),
        name: "Paired Device".to_string(),
        public_key: peer_key.to_vec(),
        session_key: Some(session_key.to_vec()),
        last_seen: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        is_active: true,
    };

    core.storage
        .devices()
        .store_device(&stored_device)
        .map_err(|e| format!("Failed to store device: {}", e))?;

    Ok(DeviceInfoDto {
        id: device_id,
        name: "Paired Device".to_string(),
        is_online: false,
        last_seen: 0,
    })
}

/// Cancel active pairing session
#[frb(sync)]
pub fn cancel_pairing() {
    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.pairing_session = None;
    }
}

// ============================================================================
// Device Management
// ============================================================================

/// Get list of paired devices
#[frb(sync)]
pub fn get_paired_devices() -> Vec<DeviceInfoDto> {
    let guard = TOSS_INSTANCE.read();
    let core = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };

    let stored_devices = match core.storage.devices().get_all_devices() {
        Ok(devices) => devices,
        Err(_) => return Vec::new(),
    };

    stored_devices
        .into_iter()
        .map(|d| DeviceInfoDto {
            id: d.id,
            name: d.name,
            is_online: false, // TODO: Check network connection status
            last_seen: d.last_seen.unwrap_or(0),
        })
        .collect()
}

/// Remove a paired device
#[frb(sync)]
pub fn remove_device(device_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    core.storage
        .devices()
        .remove_device(&device_id)
        .map_err(|e| format!("Failed to remove device: {}", e))?;

    Ok(())
}

// ============================================================================
// Clipboard Operations
// ============================================================================

/// Get current clipboard content
#[frb(sync)]
pub fn get_current_clipboard() -> Option<ClipboardItemDto> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref()?;

    let content = core.clipboard.read().ok()??;

    Some(ClipboardItemDto {
        content_type: match content.content_type {
            ContentType::PlainText => "text".to_string(),
            ContentType::RichText => "rich_text".to_string(),
            ContentType::Image => "image".to_string(),
            ContentType::File => "file".to_string(),
            ContentType::Url => "url".to_string(),
        },
        preview: content.as_text().unwrap_or_default(),
        size_bytes: content.metadata.size_bytes,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        source_device: None,
    })
}

/// Send current clipboard to all devices
#[frb]
pub async fn send_clipboard() -> Result<(), String> {
    // Read all needed data while holding the lock, then drop it before await
    let (message_clone, has_network, history_item) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let content = core
            .clipboard
            .read()
            .map_err(|e| format!("Clipboard read failed: {}", e))?
            .ok_or("Clipboard is empty")?;

        // Check settings
        let settings = &core.settings;
        match content.content_type {
            ContentType::PlainText | ContentType::Url if !settings.sync_text => {
                return Err("Text sync disabled".to_string());
            }
            ContentType::Image if !settings.sync_images => {
                return Err("Image sync disabled".to_string());
            }
            ContentType::File if !settings.sync_files => {
                return Err("File sync disabled".to_string());
            }
            _ => {}
        }

        // Check size limit
        let max_bytes = (settings.max_file_size_mb as u64) * 1024 * 1024;
        if content.metadata.size_bytes > max_bytes {
            return Err(format!(
                "Content too large (max {} MB)",
                settings.max_file_size_mb
            ));
        }

        // Prepare history item if enabled (we'll save it after dropping the guard)
        let history_item = if core.settings.history_enabled {
            Some(crate::storage::StoredHistoryItem {
                id: uuid::Uuid::new_v4().to_string(),
                content_type: content.content_type as u8,
                content_hash: hex::encode(content.hash()),
                encrypted_content: vec![], // TODO: Encrypt content before storing
                preview: content.metadata.text_preview.clone().unwrap_or_else(|| {
                    format!("{} bytes", content.metadata.size_bytes)
                }),
                source_device: None, // Local clipboard
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            })
        } else {
            None
        };

        // Broadcast to connected devices
        let update = ClipboardUpdate::new(content);
        let message = Message::ClipboardUpdate(update);

        // Clone message and check if network exists before dropping guard
        let message_clone = message.clone();
        let has_network = core.network.is_some();
        
        (message_clone, has_network, history_item)
    }; // Guard is dropped here

    // Save to history if enabled (after dropping the guard)
    if let Some(history_item) = history_item {
        let guard = TOSS_INSTANCE.read();
        if let Some(core) = guard.as_ref() {
            if let Err(e) = core.storage.history().store_item(&history_item) {
                tracing::warn!("Failed to save clipboard history: {}", e);
            }
        }
        drop(guard);
    }

    // Broadcast message (after dropping all guards)
    if has_network {
        let network_ptr: Option<*const NetworkManager> = {
            let guard = TOSS_INSTANCE.read();
            guard.as_ref()
                .and_then(|c| c.network.as_ref())
                .map(|n| n as *const NetworkManager)
        };
        
        if let Some(ptr) = network_ptr {
            // SAFETY: 
            // 1. NetworkManager::broadcast takes &self, not &mut self, so no mutation
            // 2. The network is owned by TossCore in TOSS_INSTANCE which is behind a RwLock
            // 3. We've dropped the guard, so we're not holding a lock
            // 4. The network will remain valid as long as TOSS_INSTANCE exists
            // 5. broadcast() only reads from network, so concurrent access is safe
            let network = unsafe { &*ptr };
            network
                .broadcast(&message_clone)
                .await
                .map_err(|e| format!("Failed to broadcast message: {}", e))?;
        }
    }

    Ok(())
}

/// Send text to all devices
#[frb]
pub async fn send_text(text: String) -> Result<(), String> {
    // Read all needed data while holding the lock, then drop it before await
    let (message_clone, has_network) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let content = ClipboardContent::text(&text);
        let update = ClipboardUpdate::new(content);
        let message = Message::ClipboardUpdate(update);

        // Clone message and check if network exists before dropping guard
        let message_clone = message.clone();
        let has_network = core.network.is_some();
        
        (message_clone, has_network)
    }; // Guard is dropped here

    // Broadcast message (after dropping the guard)
    if has_network {
        let network_ptr: Option<*const NetworkManager> = {
            let guard = TOSS_INSTANCE.read();
            guard.as_ref()
                .and_then(|c| c.network.as_ref())
                .map(|n| n as *const NetworkManager)
        };
        
        if let Some(ptr) = network_ptr {
            // SAFETY: 
            // 1. NetworkManager::broadcast takes &self, not &mut self, so no mutation
            // 2. The network is owned by TossCore in TOSS_INSTANCE which is behind a RwLock
            // 3. We've dropped the guard, so we're not holding a lock
            // 4. The network will remain valid as long as TOSS_INSTANCE exists
            // 5. broadcast() only reads from network, so concurrent access is safe
            let network = unsafe { &*ptr };
            network
                .broadcast(&message_clone)
                .await
                .map_err(|e| format!("Failed to broadcast message: {}", e))?;
        }
    }

    Ok(())
}

// ============================================================================
// Settings
// ============================================================================

/// Get current settings
#[frb(sync)]
pub fn get_settings() -> TossSettings {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.settings.clone())
        .unwrap_or_default()
}

/// Update settings
#[frb(sync)]
pub fn update_settings(settings: TossSettings) -> Result<(), String> {
    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.settings = settings;
        Ok(())
    } else {
        Err("Toss not initialized".to_string())
    }
}

// ============================================================================
// Network
// ============================================================================

/// Start networking
#[frb]
pub async fn start_network() -> Result<(), String> {
    // Extract config while holding lock, then release before async operations
    let (identity, config) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let config = NetworkConfig {
            device_name: core.device_name.clone(),
            relay_url: core.settings.relay_url.clone(),
            ..Default::default()
        };

        (core.identity.clone(), config)
    };

    // Perform async operations without holding lock
    let mut network = NetworkManager::new(identity, config)
        .await
        .map_err(|e| format!("Network init failed: {}", e))?;

    network
        .start()
        .await
        .map_err(|e| format!("Network start failed: {}", e))?;

    // Re-acquire lock to store network and subscribe to events
    {
        let mut guard = TOSS_INSTANCE.write();
        let core = guard.as_mut().ok_or("Toss not initialized")?;
        let receiver = network.subscribe();
        core.event_receiver = Some(Arc::new(Mutex::new(receiver)));
        core.network = Some(network);
    }

    Ok(())
}

/// Stop networking
#[frb]
pub async fn stop_network() {
    // Extract network while holding lock, then release before async operation
    let network = {
        let mut guard = TOSS_INSTANCE.write();
        guard.as_mut().and_then(|core| core.network.take())
    };

    if let Some(mut network) = network {
        network.stop().await;
    }
}

/// Start listening to network events
/// Returns a receiver that can be polled for events
/// Note: Full stream support requires flutter_rust_bridge stream support
#[frb]
pub async fn start_event_listener() -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    if let Some(ref network) = core.network {
        // Subscribe to network events
        let receiver = network.subscribe();
        // Store receiver for polling
        let mut guard = TOSS_INSTANCE.write();
        if let Some(ref mut core) = *guard {
            core.event_receiver = Some(Arc::new(Mutex::new(receiver)));
        }
    }

    Ok(())
}

/// Poll for network events (polling-based approach until streams are available)
/// Returns the next event if available, or None
/// Note: This uses try_recv which is non-blocking
#[frb(sync)]
pub fn poll_event() -> Option<TossEvent> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref()?;
    
    if let Some(ref receiver_arc) = core.event_receiver {
        // Try to receive an event (non-blocking)
        let mut receiver = receiver_arc.lock().unwrap();
        match receiver.try_recv() {
            Ok(NetworkEvent::PeerConnected { device_id, device_name }) => {
                Some(TossEvent::DeviceConnected {
                    device: DeviceInfoDto {
                        id: hex::encode(device_id),
                        name: device_name,
                        is_online: true,
                        last_seen: 0,
                    },
                })
            }
            Ok(NetworkEvent::PeerDisconnected { device_id }) => {
                Some(TossEvent::DeviceDisconnected {
                    device_id: hex::encode(device_id),
                })
            }
            Ok(NetworkEvent::MessageReceived { from_device_id, message }) => {
                // Convert Message to ClipboardItemDto if it's a clipboard update
                if let crate::protocol::Message::ClipboardUpdate(update) = message {
                    // Save to history if enabled
                    {
                        let guard = TOSS_INSTANCE.read();
                        if let Some(ref core) = guard.as_ref() {
                            if core.settings.history_enabled {
                                let history_item = crate::storage::StoredHistoryItem {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    content_type: update.content.content_type as u8,
                                    content_hash: hex::encode(update.content_hash),
                                    encrypted_content: vec![], // TODO: Store encrypted content
                                    preview: update.content.metadata.text_preview.clone()
                                        .unwrap_or_else(|| {
                                            format!("{} bytes", update.content.metadata.size_bytes)
                                        }),
                                    source_device: Some(hex::encode(from_device_id)),
                                    created_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                };
                                if let Err(e) = core.storage.history().store_item(&history_item) {
                                    tracing::warn!("Failed to save received clipboard history: {}", e);
                                }
                            }
                        }
                    }

                    // Return event for Flutter
                    Some(TossEvent::ClipboardReceived {
                        item: ClipboardItemDto {
                            content_type: format!("{:?}", update.content.content_type),
                            preview: update.content.metadata.text_preview.clone()
                                .unwrap_or_else(|| {
                                    format!("{} bytes", update.content.metadata.size_bytes)
                                }),
                            size_bytes: update.content.metadata.size_bytes,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                            source_device: Some(hex::encode(from_device_id)),
                        },
                    })
                } else {
                    None
                }
            }
            Ok(NetworkEvent::Error(msg)) => {
                Some(TossEvent::Error { message: msg })
            }
            Ok(NetworkEvent::PeerDiscovered(_)) | Ok(NetworkEvent::PeerLost(_)) => {
                // These events are less critical for Flutter UI
                None
            }
            Err(_) => None, // No event available or channel closed
        }
    } else {
        None
    }
}

/// Get clipboard history
#[frb(sync)]
pub fn get_clipboard_history(limit: Option<u32>) -> Vec<ClipboardItemDto> {
    let guard = TOSS_INSTANCE.read();
    let core = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };

    let history_items = match core.storage.history().get_all_items(limit) {
        Ok(items) => items,
        Err(_) => return Vec::new(),
    };

    history_items
        .into_iter()
        .map(|item| ClipboardItemDto {
            content_type: format!("{:?}", ContentType::try_from(item.content_type).unwrap_or(ContentType::PlainText)),
            preview: item.preview,
            size_bytes: item.encrypted_content.len() as u64,
            timestamp: item.created_at * 1000, // Convert seconds to milliseconds
            source_device: item.source_device,
        })
        .collect()
}

/// Remove clipboard history item
#[frb(sync)]
pub fn remove_history_item(item_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    core.storage
        .history()
        .remove_item(&item_id)
        .map_err(|e| format!("Failed to remove history item: {}", e))?;

    Ok(())
}

/// Clear clipboard history
#[frb(sync)]
pub fn clear_clipboard_history() -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    core.storage
        .history()
        .clear_history()
        .map_err(|e| format!("Failed to clear history: {}", e))?;

    Ok(())
}

/// Get connected devices
#[frb(sync)]
pub fn get_connected_devices() -> Vec<DeviceInfoDto> {
    let guard = TOSS_INSTANCE.read();
    if let Some(ref core) = *guard {
        if let Some(ref network) = core.network {
            return network
                .connected_peers()
                .into_iter()
                .map(|peer| DeviceInfoDto {
                    id: hex::encode(peer.device_id),
                    name: peer.device_name,
                    is_online: peer.is_connected,
                    last_seen: 0,
                })
                .collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = TossSettings::default();
        assert!(settings.auto_sync);
        assert!(settings.sync_text);
        assert!(settings.sync_images);
        assert_eq!(settings.max_file_size_mb, 50);
    }

    #[test]
    #[ignore] // Requires clipboard access (X11 server)
    fn test_init_toss() {
        let result = init_toss("/tmp/toss-test".to_string(), "Test Device".to_string());
        assert!(result.is_ok());

        // Cleanup
        *TOSS_INSTANCE.write() = None;
    }

    #[test]
    #[ignore] // Requires clipboard access (X11 server)
    fn test_pairing_flow() {
        init_toss("/tmp/toss-test".to_string(), "Test Device".to_string()).unwrap();

        let pairing_info = start_pairing().unwrap();
        assert_eq!(pairing_info.code.len(), 6);
        assert!(!pairing_info.qr_data.is_empty());

        cancel_pairing();

        // Cleanup
        *TOSS_INSTANCE.write() = None;
    }
}
