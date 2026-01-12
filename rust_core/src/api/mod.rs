//! FFI API for Flutter integration
//!
//! This module provides the public API that is exposed to Flutter
//! via flutter_rust_bridge.

use std::sync::Arc;
use parking_lot::RwLock;

use crate::clipboard::ClipboardManager;
use crate::crypto::{DeviceIdentity, PairingSession};
use crate::network::{NetworkConfig, NetworkManager};
use crate::protocol::{ClipboardContent, ClipboardUpdate, ContentType, Message};

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
}

/// Toss settings
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct DeviceInfoDto {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub last_seen: u64,
}

/// Clipboard item for display
#[derive(Debug, Clone)]
pub struct ClipboardItemDto {
    pub content_type: String,
    pub preview: String,
    pub size_bytes: u64,
    pub timestamp: u64,
    pub source_device: Option<String>,
}

/// Event types for Flutter
#[derive(Debug, Clone)]
pub enum TossEvent {
    ClipboardReceived { item: ClipboardItemDto },
    DeviceConnected { device: DeviceInfoDto },
    DeviceDisconnected { device_id: String },
    PairingRequest { device: DeviceInfoDto },
    Error { message: String },
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize Toss core
pub fn init_toss(_data_dir: String, device_name: String) -> Result<(), String> {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("toss_core=debug")
        .try_init();

    // Load or create identity
    let identity = DeviceIdentity::generate()
        .map_err(|e| format!("Failed to generate identity: {}", e))?;

    // Create clipboard manager
    let clipboard = ClipboardManager::new()
        .map_err(|e| format!("Failed to initialize clipboard: {}", e))?;

    let core = TossCore {
        identity: Arc::new(identity),
        device_name,
        clipboard,
        network: None,
        pairing_session: None,
        settings: TossSettings::default(),
    };

    *TOSS_INSTANCE.write() = Some(core);

    Ok(())
}

/// Shutdown Toss
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
pub fn get_device_id() -> String {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.identity.device_id_hex())
        .unwrap_or_default()
}

/// Get device name
pub fn get_device_name() -> String {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.device_name.clone())
        .unwrap_or_default()
}

/// Set device name
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
#[derive(Debug, Clone)]
pub struct PairingInfoDto {
    pub code: String,
    pub qr_data: String,
    pub expires_at: u64,
    pub public_key: String,
}

/// Complete pairing with QR data
pub fn complete_pairing_qr(qr_data: String) -> Result<DeviceInfoDto, String> {
    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = core.pairing_session.take()
        .ok_or("No active pairing session")?;

    let (_session_key, device_name, _public_key) = session
        .complete_from_qr(&qr_data)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    // In a real implementation, we would:
    // 1. Store the paired device info
    // 2. Establish connection using the session key

    Ok(DeviceInfoDto {
        id: "paired-device".to_string(), // Would be derived from public key
        name: device_name,
        is_online: false,
        last_seen: 0,
    })
}

/// Complete pairing with manual code
pub fn complete_pairing_code(
    code: String,
    peer_public_key: Vec<u8>,
) -> Result<DeviceInfoDto, String> {
    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = core.pairing_session.take()
        .ok_or("No active pairing session")?;

    let peer_key: [u8; 32] = peer_public_key
        .try_into()
        .map_err(|_| "Invalid public key length")?;

    let _session_key = session
        .complete(&peer_key, &code)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    Ok(DeviceInfoDto {
        id: hex::encode(&peer_key[..16]),
        name: "Paired Device".to_string(),
        is_online: false,
        last_seen: 0,
    })
}

/// Cancel active pairing session
pub fn cancel_pairing() {
    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.pairing_session = None;
    }
}

// ============================================================================
// Device Management
// ============================================================================

/// Get list of paired devices
pub fn get_paired_devices() -> Vec<DeviceInfoDto> {
    // In a real implementation, this would read from storage
    Vec::new()
}

/// Remove a paired device
pub fn remove_device(_device_id: String) -> Result<(), String> {
    // In a real implementation, this would remove from storage
    Ok(())
}

// ============================================================================
// Clipboard Operations
// ============================================================================

/// Get current clipboard content
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
pub async fn send_clipboard() -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let content = core.clipboard.read()
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
        return Err(format!("Content too large (max {} MB)", settings.max_file_size_mb));
    }

    // In a real implementation, broadcast to connected devices
    let update = ClipboardUpdate::new(content);
    let _message = Message::ClipboardUpdate(update);

    // network.broadcast(&message).await

    Ok(())
}

/// Send text to all devices
pub async fn send_text(text: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let _core = guard.as_ref().ok_or("Toss not initialized")?;

    let content = ClipboardContent::text(&text);
    let update = ClipboardUpdate::new(content);
    let _message = Message::ClipboardUpdate(update);

    // TODO: network.broadcast(&message).await

    Ok(())
}

// ============================================================================
// Settings
// ============================================================================

/// Get current settings
pub fn get_settings() -> TossSettings {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.settings.clone())
        .unwrap_or_default()
}

/// Update settings
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

    network.start()
        .await
        .map_err(|e| format!("Network start failed: {}", e))?;

    // Re-acquire lock to store network
    {
        let mut guard = TOSS_INSTANCE.write();
        let core = guard.as_mut().ok_or("Toss not initialized")?;
        core.network = Some(network);
    }

    Ok(())
}

/// Stop networking
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

/// Get connected devices
pub fn get_connected_devices() -> Vec<DeviceInfoDto> {
    let guard = TOSS_INSTANCE.read();
    if let Some(ref core) = *guard {
        if let Some(ref network) = core.network {
            return network.connected_peers()
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
    fn test_init_toss() {
        let result = init_toss("/tmp/toss-test".to_string(), "Test Device".to_string());
        assert!(result.is_ok());

        // Cleanup
        *TOSS_INSTANCE.write() = None;
    }

    #[test]
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
