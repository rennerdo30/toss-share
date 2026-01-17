//! FFI API for Flutter integration
//!
//! This module provides the public API that is exposed to Flutter
//! via flutter_rust_bridge.

use flutter_rust_bridge::frb;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::clipboard::ClipboardManager;
use crate::crypto::{
    decrypt, derive_key, encrypt, DerivedKeyPurpose, DeviceIdentity, EncryptedMessage,
    PairingSession,
};
use crate::network::{GetSessionKeyFn, NetworkConfig, NetworkEvent, NetworkManager};
use crate::protocol::{ClipboardContent, ClipboardUpdate, ContentType, Message};
use crate::storage::{Storage, StoredDevice};

/// Global Toss instance
static TOSS_INSTANCE: RwLock<Option<TossCore>> = RwLock::new(None);

/// Guard for file logger (must be kept alive for logging to work)
static LOG_GUARD: RwLock<Option<WorkerGuard>> = RwLock::new(None);

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
    last_sync_time: std::sync::Mutex<std::time::Instant>,
}

/// Toss settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TossSettings {
    pub auto_sync: bool,
    pub sync_text: bool,
    pub sync_rich_text: bool,
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
            sync_rich_text: true,
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
    pub platform: String, // Platform name: "macos", "windows", "linux", "ios", "android", "unknown"
}

/// Clipboard item for display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardItemDto {
    pub id: String, // Unique identifier for history item
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
    // Initialize file-based logging
    let log_dir = std::path::Path::new(&data_dir).join("logs");
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create log directory: {}", e))?;

    let file_appender =
        tracing_appender::rolling::daily(&log_dir, "toss.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard to keep logging active
    *LOG_GUARD.write() = Some(guard);

    // Initialize tracing subscriber with both stdout and file output
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .with(tracing_subscriber::EnvFilter::new("toss_core=debug"))
        .try_init();

    tracing::info!("Toss core initializing with data_dir: {}", data_dir);

    // Initialize storage
    let db_path = std::path::Path::new(&data_dir).join("toss.db");
    let storage =
        Storage::new(&db_path).map_err(|e| format!("Failed to initialize storage: {}", e))?;

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
        last_sync_time: std::sync::Mutex::new(std::time::Instant::now()),
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
    // Validate device name
    let name = name.trim();
    if name.is_empty() {
        return Err("Device name cannot be empty".to_string());
    }
    if name.len() > 100 {
        return Err("Device name too long (max 100 characters)".to_string());
    }

    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.device_name = name.to_string();
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
    // Validate QR data
    let qr_data = qr_data.trim();
    if qr_data.is_empty() {
        return Err("QR data cannot be empty".to_string());
    }
    if qr_data.len() > 1000 {
        return Err("QR data too long (max 1000 characters)".to_string());
    }

    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    let session = core
        .pairing_session
        .take()
        .ok_or("No active pairing session")?;

    let (session_key, device_name, public_key_base64) = session
        .complete_from_qr(qr_data)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    // Decode public key from base64
    let public_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &public_key_base64,
    )
    .map_err(|e| format!("Invalid public key: {}", e))?;

    // Derive device ID from public key hash
    let device_id = hex::encode(&Sha256::digest(&public_key)[..16]);

    // Encrypt session key before storing
    let encrypted_session_key = {
        let storage_key = derive_key(
            core.identity.device_id() as &[u8],
            DerivedKeyPurpose::StorageEncryption,
            Some(b"toss-session-key-v1"),
        )
        .map_err(|e| format!("Failed to derive storage key: {}", e))?;

        let aad = format!("session:{}", device_id).into_bytes();
        let encrypted = encrypt(&storage_key, &session_key, &aad)
            .map_err(|e| format!("Failed to encrypt session key: {}", e))?;
        Some(encrypted.to_bytes())
    };

    // Store the paired device
    let stored_device = StoredDevice {
        id: device_id.clone(),
        name: device_name.clone(),
        public_key,
        session_key: encrypted_session_key,
        last_seen: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        is_active: true,
        platform: Some(format!("{:?}", crate::protocol::Platform::current()).to_lowercase()),
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
        platform: format!("{:?}", crate::protocol::Platform::current()).to_lowercase(),
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

    // Encrypt session key before storing
    let encrypted_session_key = {
        let storage_key = derive_key(
            core.identity.device_id() as &[u8],
            DerivedKeyPurpose::StorageEncryption,
            Some(b"toss-session-key-v1"),
        )
        .map_err(|e| format!("Failed to derive storage key: {}", e))?;

        let aad = format!("session:{}", device_id).into_bytes();
        let encrypted = encrypt(&storage_key, &session_key, &aad)
            .map_err(|e| format!("Failed to encrypt session key: {}", e))?;
        Some(encrypted.to_bytes())
    };

    // Store the paired device
    let stored_device = StoredDevice {
        id: device_id.clone(),
        name: "Paired Device".to_string(),
        public_key: peer_key.to_vec(),
        session_key: encrypted_session_key,
        last_seen: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        is_active: true,
        platform: Some(format!("{:?}", crate::protocol::Platform::current()).to_lowercase()),
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
        platform: format!("{:?}", crate::protocol::Platform::current()).to_lowercase(),
    })
}

/// Cancel active pairing session
#[frb(sync)]
pub fn cancel_pairing() {
    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.pairing_session = None;
    }
}

/// Pairing device info returned from find_pairing_device
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairingDeviceDto {
    pub code: String,
    pub public_key: String, // Base64 encoded
    pub device_name: String,
    pub via_relay: bool,
}

/// Find a device by pairing code (searches mDNS and relay server)
#[frb]
pub async fn find_pairing_device(code: String) -> Result<PairingDeviceDto, String> {
    // Validate code format
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Pairing code must be 6 digits".to_string());
    }

    // Get relay URL and device name from settings
    let (relay_url, device_name) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;
        (core.settings.relay_url.clone(), core.device_name.clone())
    };

    // Create pairing coordinator
    let coordinator = crate::pairing::PairingCoordinator::new(&device_name, relay_url)
        .map_err(|e| format!("Failed to create pairing coordinator: {}", e))?;

    // Find device
    let device_info = coordinator
        .find_device(&code)
        .await
        .map_err(|e| format!("Failed to find device: {}", e))?;

    // Encode public key as base64
    let public_key = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &device_info.public_key,
    );

    Ok(PairingDeviceDto {
        code: device_info.code,
        public_key,
        device_name: device_info.device_name,
        via_relay: device_info.via_relay,
    })
}

/// Complete pairing with a device found via find_pairing_device
#[frb(sync)]
pub fn complete_manual_pairing(
    peer_public_key: String,
    peer_device_name: String,
) -> Result<DeviceInfoDto, String> {
    // Decode public key from base64
    let public_key_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &peer_public_key,
    )
    .map_err(|e| format!("Invalid public key encoding: {}", e))?;

    if public_key_bytes.len() != 32 {
        return Err("Invalid public key length (expected 32 bytes)".to_string());
    }

    let mut peer_key = [0u8; 32];
    peer_key.copy_from_slice(&public_key_bytes);

    let mut guard = TOSS_INSTANCE.write();
    let core = guard.as_mut().ok_or("Toss not initialized")?;

    // Get or create a pairing session
    let session = core
        .pairing_session
        .take()
        .unwrap_or_else(|| PairingSession::new(&core.device_name));

    // Complete the pairing using X25519 key exchange
    let session_key = session
        .complete_with_peer_key(&peer_key)
        .map_err(|e| format!("Pairing failed: {}", e))?;

    // Derive device ID from public key
    let device_id = hex::encode(&Sha256::digest(&peer_key)[..16]);

    // Encrypt session key before storing
    let encrypted_session_key = {
        let storage_key = derive_key(
            core.identity.device_id() as &[u8],
            DerivedKeyPurpose::StorageEncryption,
            Some(b"toss-session-key-v1"),
        )
        .map_err(|e| format!("Failed to derive storage key: {}", e))?;

        let aad = format!("session:{}", device_id).into_bytes();
        let encrypted = encrypt(&storage_key, &session_key, &aad)
            .map_err(|e| format!("Failed to encrypt session key: {}", e))?;
        Some(encrypted.to_bytes())
    };

    // Store the paired device
    let stored_device = StoredDevice {
        id: device_id.clone(),
        name: peer_device_name.clone(),
        public_key: peer_key.to_vec(),
        session_key: encrypted_session_key,
        last_seen: None,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        is_active: true,
        platform: Some("unknown".to_string()), // Platform not available from pairing info
    };

    core.storage
        .devices()
        .store_device(&stored_device)
        .map_err(|e| format!("Failed to store device: {}", e))?;

    Ok(DeviceInfoDto {
        id: device_id,
        name: peer_device_name,
        is_online: false,
        last_seen: 0,
        platform: "unknown".to_string(),
    })
}

/// Register pairing code on relay server and via mDNS
#[frb]
pub async fn register_pairing_advertisement() -> Result<(), String> {
    // Get current pairing session, relay URL, and device name
    let (code, public_key, relay_url, device_name) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let session = core
            .pairing_session
            .as_ref()
            .ok_or("No active pairing session")?;

        let info = session.info(&core.device_name);
        let public_key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &info.public_key,
        )
        .map_err(|e| format!("Invalid public key: {}", e))?;

        let mut pk = [0u8; 32];
        pk.copy_from_slice(&public_key_bytes);

        (info.code, pk, core.settings.relay_url.clone(), core.device_name.clone())
    };

    // Create pairing coordinator and start advertisement
    let coordinator = crate::pairing::PairingCoordinator::new(&device_name, relay_url)
        .map_err(|e| format!("Failed to create pairing coordinator: {}", e))?;

    coordinator
        .start_advertisement(&code, &public_key)
        .await
        .map_err(|e| format!("Failed to start advertisement: {}", e))?;

    Ok(())
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

    // Get list of connected device IDs from network
    let connected_device_ids: std::collections::HashSet<String> =
        if let Some(ref network) = core.network {
            network
                .connected_peers()
                .into_iter()
                .map(|peer| hex::encode(peer.device_id))
                .collect()
        } else {
            std::collections::HashSet::new()
        };

    stored_devices
        .into_iter()
        .map(|d| DeviceInfoDto {
            id: d.id.clone(),
            name: d.name,
            is_online: connected_device_ids.contains(&d.id),
            last_seen: d.last_seen.unwrap_or(0),
            platform: d.platform.unwrap_or_else(|| "unknown".to_string()),
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

/// Rename a paired device
#[frb(sync)]
pub fn rename_device(device_id: String, new_name: String) -> Result<(), String> {
    // Validate device name
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("Device name cannot be empty".to_string());
    }
    if new_name.len() > 100 {
        return Err("Device name too long (max 100 characters)".to_string());
    }

    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    core.storage
        .devices()
        .update_device_name(&device_id, new_name)
        .map_err(|e| format!("Failed to rename device: {}", e))?;

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
        id: uuid::Uuid::new_v4().to_string(),
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
    // Rate limiting: prevent rapid-fire syncs (minimum 100ms between syncs)
    {
        let guard = TOSS_INSTANCE.read();
        if let Some(core) = guard.as_ref() {
            let last_sync = core.last_sync_time.lock().unwrap();
            let elapsed = last_sync.elapsed();
            if elapsed.as_millis() < 100 {
                return Err(format!(
                    "Rate limit: please wait {}ms",
                    100 - elapsed.as_millis()
                ));
            }
        }
    }

    // Read all needed data while holding the lock, then drop it before await
    let (
        message_clone,
        has_network,
        history_item,
        content_data_for_encryption,
        identity_for_encryption,
    ) = {
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
            ContentType::RichText if !settings.sync_rich_text => {
                return Err("Rich text sync disabled".to_string());
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
        // Note: Encryption will happen when saving, not here, to avoid holding lock during crypto ops
        let (history_item, content_data_for_encryption, identity_for_encryption) =
            if core.settings.history_enabled {
                let item_id = uuid::Uuid::new_v4().to_string();
                match bincode::serialize(&content) {
                    Ok(content_data) => {
                        let identity = core.identity.clone();
                        let history_item = crate::storage::StoredHistoryItem {
                            id: item_id.clone(),
                            content_type: content.content_type as u8,
                            content_hash: hex::encode(content.hash()),
                            encrypted_content: vec![], // Will be populated after encryption
                            preview: content.metadata.text_preview.clone().unwrap_or_else(|| {
                                format!("{} bytes", content.metadata.size_bytes)
                            }),
                            source_device: None, // Local clipboard
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        };
                        (
                            Some(history_item),
                            Some((item_id, content_data)),
                            Some(identity),
                        )
                    }
                    Err(e) => {
                        tracing::warn!("Failed to serialize content for history: {}", e);
                        // Skip history if serialization fails, but continue with sending
                        (None, None, None)
                    }
                }
            } else {
                (None, None, None)
            };

        // Broadcast to connected devices
        let update = ClipboardUpdate::new(content);
        let message = Message::ClipboardUpdate(update);

        // Clone message and check if network exists before dropping guard
        let message_clone = message.clone();
        let has_network = core.network.is_some();

        (
            message_clone,
            has_network,
            history_item,
            content_data_for_encryption,
            identity_for_encryption,
        )
    }; // Guard is dropped here

    // Encrypt and save to history if enabled (after dropping the guard)
    if let (Some(mut history_item), Some((item_id, content_data)), Some(identity)) = (
        history_item,
        content_data_for_encryption,
        identity_for_encryption,
    ) {
        // Derive storage encryption key
        if let Ok(storage_key) = derive_key(
            identity.device_id() as &[u8],
            DerivedKeyPurpose::StorageEncryption,
            Some(b"toss-clipboard-history-v1"),
        ) {
            // Encrypt content
            let aad = format!("history:{}", item_id).into_bytes();
            if let Ok(encrypted) = encrypt(&storage_key, &content_data, &aad) {
                history_item.encrypted_content = encrypted.to_bytes();

                // Save to storage
                let guard = TOSS_INSTANCE.read();
                if let Some(core) = guard.as_ref() {
                    if let Err(e) = core.storage.history().store_item(&history_item) {
                        tracing::warn!("Failed to save clipboard history: {}", e);
                    }
                }
            } else {
                tracing::warn!("Failed to encrypt clipboard history content");
            }
        } else {
            tracing::warn!("Failed to derive storage key for clipboard history");
        }
    }

    // Broadcast message (after dropping all guards)
    if has_network {
        let network_ptr: Option<*const NetworkManager> = {
            let guard = TOSS_INSTANCE.read();
            guard
                .as_ref()
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
            guard
                .as_ref()
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
    let (identity, config, get_session_key) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let config = NetworkConfig {
            device_name: core.device_name.clone(),
            relay_url: core.settings.relay_url.clone(),
            ..Default::default()
        };

        // Create a callback to retrieve session keys from storage
        // We need to capture the storage database path for later use
        let db_path = core.storage.db_path().to_path_buf();
        let get_session_key: Arc<GetSessionKeyFn> =
            Arc::new(Box::new(move |device_id: &[u8; 32]| {
                // Open a temporary connection to look up the session key
                if let Ok(storage) = Storage::new(&db_path) {
                    let device_id_hex = hex::encode(device_id);
                    if let Ok(Some(device)) = storage.devices().get_device(&device_id_hex) {
                        if let Some(session_key_bytes) = device.session_key {
                            if session_key_bytes.len() == 32 {
                                let mut key = [0u8; 32];
                                key.copy_from_slice(&session_key_bytes);
                                return Some(key);
                            }
                        }
                    }
                }
                None
            }));

        (core.identity.clone(), config, get_session_key)
    };

    // Perform async operations without holding lock
    let mut network =
        NetworkManager::new_with_callbacks(identity, config, None, Some(get_session_key))
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
            Ok(NetworkEvent::PeerConnected {
                device_id,
                device_name,
            }) => {
                Some(TossEvent::DeviceConnected {
                    device: DeviceInfoDto {
                        id: hex::encode(device_id),
                        name: device_name,
                        is_online: true,
                        last_seen: 0,
                        platform: "unknown".to_string(), // Platform info not available in event yet
                    },
                })
            }
            Ok(NetworkEvent::PeerDisconnected { device_id }) => {
                Some(TossEvent::DeviceDisconnected {
                    device_id: hex::encode(device_id),
                })
            }
            Ok(NetworkEvent::MessageReceived {
                from_device_id,
                message,
            }) => {
                // Verify that the message is from a paired device and not from ourselves
                let (is_paired, is_self) = {
                    let guard = TOSS_INSTANCE.read();
                    if let Some(core) = guard.as_ref() {
                        let device_id_str = hex::encode(from_device_id);
                        let is_paired = matches!(
                            core.storage.devices().get_device(&device_id_str),
                            Ok(Some(_))
                        );
                        let is_self = from_device_id == *core.identity.device_id();
                        (is_paired, is_self)
                    } else {
                        (false, false)
                    }
                };

                // Only process messages from paired devices (not ourselves)
                if !is_paired {
                    tracing::warn!(
                        "Received message from unpaired device: {}",
                        hex::encode(from_device_id)
                    );
                    return None;
                }

                // Ignore messages from ourselves to prevent self-sync loops
                if is_self {
                    tracing::debug!("Ignoring message from self");
                    return None;
                }

                // Convert Message to ClipboardItemDto if it's a clipboard update
                if let crate::protocol::Message::ClipboardUpdate(update) = message {
                    // Validate content hash to ensure integrity
                    let computed_hash = update.content.hash();
                    if computed_hash != update.content_hash {
                        tracing::warn!(
                            "Content hash mismatch for received clipboard update from device {}",
                            hex::encode(from_device_id)
                        );
                        // Continue anyway - hash mismatch might be due to serialization differences
                        // In production, this should probably reject the message
                    }

                    // Validate content size limit
                    let max_size = {
                        let guard = TOSS_INSTANCE.read();
                        if let Some(core) = guard.as_ref() {
                            (core.settings.max_file_size_mb as u64) * 1024 * 1024
                        } else {
                            return None;
                        }
                    };

                    if update.content.metadata.size_bytes > max_size {
                        tracing::warn!("Received clipboard content exceeds size limit ({} bytes > {} bytes) from device {}", 
                            update.content.metadata.size_bytes, max_size, hex::encode(from_device_id));
                        return None;
                    }

                    // Check settings and write to clipboard if sync is enabled for this content type
                    let should_write = {
                        let guard = TOSS_INSTANCE.read();
                        if let Some(core) = guard.as_ref() {
                            let settings = &core.settings;
                            match update.content.content_type {
                                ContentType::PlainText | ContentType::Url => settings.sync_text,
                                ContentType::RichText => settings.sync_rich_text,
                                ContentType::Image => settings.sync_images,
                                ContentType::File => settings.sync_files,
                            }
                        } else {
                            false
                        }
                    };

                    // Write to clipboard if sync is enabled for this content type
                    if should_write {
                        let mut guard = TOSS_INSTANCE.write();
                        if let Some(ref mut core) = guard.as_mut() {
                            if let Err(e) = core.clipboard.write(&update.content) {
                                tracing::warn!("Failed to write received clipboard content: {}", e);
                            } else {
                                // Update monitor hash to prevent re-syncing this content
                                core.clipboard.monitor_mut().update_hash(&update.content);
                            }
                        }
                    }

                    // Save to history if enabled (with encryption)
                    {
                        let guard = TOSS_INSTANCE.read();
                        if let Some(core) = guard.as_ref() {
                            if core.settings.history_enabled {
                                let item_id = uuid::Uuid::new_v4().to_string();
                                let content_data = match bincode::serialize(&update.content) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to serialize received content for history: {}",
                                            e
                                        );
                                        // Skip history if serialization fails
                                        return Some(TossEvent::ClipboardReceived {
                                            item: ClipboardItemDto {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                content_type: format!(
                                                    "{:?}",
                                                    update.content.content_type
                                                ),
                                                preview: update
                                                    .content
                                                    .metadata
                                                    .text_preview
                                                    .clone()
                                                    .unwrap_or_else(|| {
                                                        format!(
                                                            "{} bytes",
                                                            update.content.metadata.size_bytes
                                                        )
                                                    }),
                                                size_bytes: update.content.metadata.size_bytes,
                                                timestamp: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_millis()
                                                    as u64,
                                                source_device: Some(hex::encode(from_device_id)),
                                            },
                                        });
                                    }
                                };

                                // Derive storage encryption key
                                if let Ok(storage_key) = derive_key(
                                    core.identity.device_id(),
                                    DerivedKeyPurpose::StorageEncryption,
                                    Some(b"toss-clipboard-history-v1"),
                                ) {
                                    // Encrypt content
                                    let aad = format!("history:{}", item_id).into_bytes();
                                    if let Ok(encrypted) =
                                        encrypt(&storage_key, &content_data, &aad)
                                    {
                                        let history_item = crate::storage::StoredHistoryItem {
                                            id: item_id,
                                            content_type: update.content.content_type as u8,
                                            content_hash: hex::encode(update.content_hash),
                                            encrypted_content: encrypted.to_bytes(),
                                            preview: update
                                                .content
                                                .metadata
                                                .text_preview
                                                .clone()
                                                .unwrap_or_else(|| {
                                                    format!(
                                                        "{} bytes",
                                                        update.content.metadata.size_bytes
                                                    )
                                                }),
                                            source_device: Some(hex::encode(from_device_id)),
                                            created_at: std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                        };
                                        if let Err(e) =
                                            core.storage.history().store_item(&history_item)
                                        {
                                            tracing::warn!(
                                                "Failed to save received clipboard history: {}",
                                                e
                                            );
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Failed to encrypt received clipboard history content"
                                        );
                                    }
                                } else {
                                    tracing::warn!("Failed to derive storage key for received clipboard history");
                                }
                            }
                        }
                    }

                    // Return event for Flutter
                    Some(TossEvent::ClipboardReceived {
                        item: ClipboardItemDto {
                            id: uuid::Uuid::new_v4().to_string(),
                            content_type: format!("{:?}", update.content.content_type),
                            preview: update.content.metadata.text_preview.clone().unwrap_or_else(
                                || format!("{} bytes", update.content.metadata.size_bytes),
                            ),
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
            Ok(NetworkEvent::Error(msg)) => Some(TossEvent::Error { message: msg }),
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
            id: item.id,
            content_type: format!(
                "{:?}",
                ContentType::try_from(item.content_type).unwrap_or(ContentType::PlainText)
            ),
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
                    platform: "unknown".to_string(), // Platform info not available in PeerInfo yet
                })
                .collect();
        }
    }
    Vec::new()
}

/// Decrypt and retrieve session key for a paired device
/// This is used internally when establishing connections with stored devices
#[frb(sync)]
pub fn get_device_session_key(device_id: String) -> Result<Vec<u8>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    // Get stored device
    let device = core
        .storage
        .devices()
        .get_device(&device_id)
        .map_err(|e| format!("Failed to get device: {}", e))?
        .ok_or("Device not found")?;

    // Check if session key exists
    let encrypted_session_key = device
        .session_key
        .ok_or("No session key stored for this device")?;

    // Derive storage decryption key
    let storage_key = derive_key(
        core.identity.device_id(),
        DerivedKeyPurpose::StorageEncryption,
        Some(b"toss-session-key-v1"),
    )
    .map_err(|e| format!("Failed to derive storage key: {}", e))?;

    // Decrypt session key
    let aad = format!("session:{}", device_id).into_bytes();
    let encrypted_message = EncryptedMessage::from_bytes(&encrypted_session_key)
        .map_err(|e| format!("Failed to parse encrypted session key: {}", e))?;

    let decrypted_key = decrypt(&storage_key, &encrypted_message, &aad)
        .map_err(|e| format!("Failed to decrypt session key: {}", e))?;

    Ok(decrypted_key)
}

/// Check if clipboard has changed since last check
#[frb(sync)]
pub fn check_clipboard_changed() -> bool {
    let mut guard = TOSS_INSTANCE.write();
    if let Some(ref mut core) = *guard {
        core.clipboard.has_changed()
    } else {
        false
    }
}

/// Decrypted clipboard content from history
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardContentDto {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Get decrypted clipboard content from history item
#[frb(sync)]
pub fn get_clipboard_history_content(item_id: String) -> Result<ClipboardContentDto, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    // Get stored history item
    let stored_item = core
        .storage
        .history()
        .get_item(&item_id)
        .map_err(|e| format!("Failed to get history item: {}", e))?
        .ok_or("History item not found")?;

    // Derive storage decryption key
    let storage_key = derive_key(
        core.identity.device_id().as_slice(),
        DerivedKeyPurpose::StorageEncryption,
        Some(b"toss-clipboard-history-v1"),
    )
    .map_err(|e| format!("Failed to derive storage key: {}", e))?;

    // Decrypt content
    let aad = format!("history:{}", item_id).into_bytes();
    let encrypted_message = EncryptedMessage::from_bytes(&stored_item.encrypted_content)
        .map_err(|e| format!("Failed to parse encrypted content: {}", e))?;

    let decrypted_data = decrypt(&storage_key, &encrypted_message, &aad)
        .map_err(|e| format!("Failed to decrypt history content: {}", e))?;

    // Deserialize to ClipboardContent to get the actual data
    let content: ClipboardContent = bincode::deserialize(&decrypted_data)
        .map_err(|e| format!("Failed to deserialize clipboard content: {}", e))?;

    // Convert content type to string
    let content_type_str = match content.content_type {
        ContentType::PlainText => "text".to_string(),
        ContentType::RichText => "rich_text".to_string(),
        ContentType::Image => "image".to_string(),
        ContentType::File => "file".to_string(),
        ContentType::Url => "url".to_string(),
    };

    Ok(ClipboardContentDto {
        content_type: content_type_str,
        data: content.data,
    })
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
