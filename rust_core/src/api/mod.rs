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
use crate::network::{
    ConnectionType, GetSessionKeyFn, NetworkConfig, NetworkEvent, NetworkManager,
};
use crate::protocol::{ClipboardContent, ClipboardUpdate, ContentType, Message};
use crate::storage::{Storage, StoredDevice};

/// Get current unix timestamp in seconds.
/// Returns 0 if system time is before UNIX_EPOCH (should never happen on properly configured systems).
fn current_unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|e| {
            tracing::error!("System time before UNIX_EPOCH: {}", e);
            0
        })
}

/// Get current unix timestamp in milliseconds.
/// Returns 0 if system time is before UNIX_EPOCH (should never happen on properly configured systems).
fn current_unix_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_else(|e| {
            tracing::error!("System time before UNIX_EPOCH: {}", e);
            0
        })
}

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
    /// Chunk size for streaming large content (bytes)
    /// Default: 1 MB (1048576), Min: 64 KB, Max: 4 MB
    pub streaming_chunk_size: u32,
    /// Enable chunked streaming for large content (> 1 MB)
    pub streaming_enabled: bool,
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
            streaming_chunk_size: crate::protocol::DEFAULT_CHUNK_SIZE as u32,
            streaming_enabled: true,
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
    pub connection_type: String, // Connection type: "direct", "stun_reflexive", "turn_relay", "websocket_relay", "unknown"
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
    // Create log directory and install panic hook FIRST
    // This ensures we can capture any panics during initialization
    let log_dir = std::path::Path::new(&data_dir).join("logs");
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| format!("Failed to create log directory: {}", e))?;

    // Install panic hook before any other initialization
    crate::panic_handler::install_panic_hook(&log_dir);

    // Initialize file-based logging
    let file_appender = tracing_appender::rolling::daily(&log_dir, "toss.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard to keep logging active
    *LOG_GUARD.write() = Some(guard);

    // Initialize tracing subscriber with both stdout and file output
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("toss_core=debug"));

    match tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .try_init()
    {
        Ok(_) => tracing::info!("Toss core initializing with data_dir: {}", data_dir),
        Err(e) => eprintln!(
            "Warning: tracing init failed (may already be initialized): {}",
            e
        ),
    }

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

    // Load settings from database, fall back to defaults if not found
    let settings = storage.load_settings().unwrap_or_else(|e| {
        tracing::warn!(
            "Failed to load settings from database: {}, using defaults",
            e
        );
        TossSettings::default()
    });

    // Run cleanup on startup if history is enabled
    if settings.history_enabled {
        match storage.cleanup_old_history(settings.history_days) {
            Ok(deleted) => {
                if deleted > 0 {
                    tracing::info!(
                        "Startup cleanup: removed {} old history entries (retention: {} days)",
                        deleted,
                        settings.history_days
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Startup cleanup failed: {}", e);
            }
        }
    }

    let core = TossCore {
        identity: Arc::new(identity),
        device_name,
        clipboard,
        network: None,
        pairing_session: None,
        settings,
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
//
// Pairing API Overview:
// ---------------------
// This module provides three pairing completion methods for different use cases:
//
// 1. `complete_pairing_qr(qr_data)` - For QR code scanning
//    Use when one device displays a QR code and the other scans it.
//    The QR contains the public key and device info.
//
// 2. `complete_pairing_code(code, peer_public_key)` - For manual code entry
//    Use when devices can't scan QR codes. The user enters the 6-digit code
//    displayed on the other device, and the public key is exchanged via
//    local network discovery.
//
// 3. `complete_manual_pairing(peer_public_key, peer_device_name)` - For network discovery
//    Use when devices are discovered via mDNS or relay server. The public key
//    and device name are obtained from `find_pairing_device()`.
//
// Typical flows:
// - QR flow: start_pairing() -> display QR -> other device scans -> complete_pairing_qr()
// - Code flow: start_pairing() -> display code -> enter on other device -> complete_pairing_code()
// - Discovery flow: start_pairing() -> register_pairing_advertisement() -> find_pairing_device() -> complete_manual_pairing()

/// Start a new pairing session.
///
/// Creates a new pairing session and returns the pairing info (code, QR data, public key).
/// The session must be completed using one of the `complete_pairing_*` functions.
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

/// Complete pairing with QR data.
///
/// Use this when the other device has scanned your QR code or you have scanned theirs.
/// The QR data contains the peer's public key, device name, and pairing code.
///
/// # Arguments
/// * `qr_data` - The scanned QR code data containing pairing information
///
/// # Returns
/// Device info for the newly paired device
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
        created_at: current_unix_timestamp_secs(),
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
        connection_type: "unknown".to_string(),
    })
}

/// Complete pairing with manual code entry.
///
/// Use this when the user manually enters the 6-digit pairing code displayed
/// on the other device. The peer's public key is obtained separately
/// (typically via local network discovery or relay server).
///
/// # Arguments
/// * `code` - The 6-digit pairing code from the other device
/// * `peer_public_key` - The peer's public key (32 bytes)
///
/// # Returns
/// Device info for the newly paired device
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
        created_at: current_unix_timestamp_secs(),
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
        connection_type: "unknown".to_string(),
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

/// Result of pairing advertisement registration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvertisementResultDto {
    /// Whether mDNS registration succeeded
    pub mdns_registered: bool,
    /// Whether relay server registration succeeded
    pub relay_registered: bool,
    /// Error message if mDNS registration failed
    pub mdns_error: Option<String>,
    /// Error message if relay registration failed
    pub relay_error: Option<String>,
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

/// Complete pairing with a device found via network discovery.
///
/// Use this after finding a device with `find_pairing_device()`. This is the
/// final step in the discovery-based pairing flow where devices find each other
/// via mDNS or relay server.
///
/// # Arguments
/// * `peer_public_key` - Base64-encoded public key from `PairingDeviceDto`
/// * `peer_device_name` - Device name from `PairingDeviceDto`
///
/// # Returns
/// Device info for the newly paired device
#[frb(sync)]
pub fn complete_manual_pairing(
    peer_public_key: String,
    peer_device_name: String,
) -> Result<DeviceInfoDto, String> {
    // Decode public key from base64
    let public_key_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &peer_public_key)
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
        created_at: current_unix_timestamp_secs(),
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
        connection_type: "unknown".to_string(),
    })
}

/// Register pairing code on relay server and via mDNS
/// Returns the result indicating which methods succeeded/failed
#[frb]
pub async fn register_pairing_advertisement() -> Result<AdvertisementResultDto, String> {
    // Get current pairing session, relay URL, and device name
    let (code, public_key, relay_url, device_name) = {
        let guard = TOSS_INSTANCE.read();
        let core = guard.as_ref().ok_or("Toss not initialized")?;

        let session = core
            .pairing_session
            .as_ref()
            .ok_or("No active pairing session")?;

        let info = session.info(&core.device_name);
        let public_key_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &info.public_key)
                .map_err(|e| format!("Invalid public key: {}", e))?;

        let mut pk = [0u8; 32];
        pk.copy_from_slice(&public_key_bytes);

        (
            info.code,
            pk,
            core.settings.relay_url.clone(),
            core.device_name.clone(),
        )
    };

    // Create pairing coordinator and start advertisement
    let coordinator = crate::pairing::PairingCoordinator::new(&device_name, relay_url)
        .map_err(|e| format!("Failed to create pairing coordinator: {}", e))?;

    let result = coordinator
        .start_advertisement(&code, &public_key)
        .await
        .map_err(|e| format!("Failed to start advertisement: {}", e))?;

    Ok(AdvertisementResultDto {
        mdns_registered: result.mdns_registered,
        relay_registered: result.relay_registered,
        mdns_error: result.mdns_error,
        relay_error: result.relay_error,
    })
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
            connection_type: "unknown".to_string(), // Connection type not stored
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
        timestamp: current_unix_timestamp_millis(),
        source_device: None,
    })
}

/// Send current clipboard to all devices
/// For large content (> 1MB when streaming enabled), uses chunked transfer protocol
#[frb]
pub async fn send_clipboard() -> Result<(), String> {
    // Rate limiting: prevent rapid-fire syncs (minimum 100ms between syncs)
    {
        let guard = TOSS_INSTANCE.read();
        if let Some(core) = guard.as_ref() {
            let last_sync = core
                .last_sync_time
                .lock()
                .expect("last_sync_time mutex poisoned - this is a bug");
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
        content_for_send,
        use_chunked,
        chunk_size,
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

        // Determine if we should use chunked transfer
        let use_chunked = settings.streaming_enabled
            && content.data.len() > crate::protocol::CHUNKED_TRANSFER_THRESHOLD;
        let chunk_size = settings.streaming_chunk_size as usize;

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
                            created_at: current_unix_timestamp_secs(),
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

        let has_network = core.network.is_some();

        (
            content,
            use_chunked,
            chunk_size,
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

            if use_chunked {
                // Use chunked transfer for large content
                tracing::info!(
                    "Using chunked transfer for {} bytes content (chunk size: {})",
                    content_for_send.data.len(),
                    chunk_size
                );

                // Create transfer init and chunks
                let init = crate::protocol::ChunkedTransferInit::new(&content_for_send, chunk_size);
                let total_chunks = init.total_chunks;
                let transfer_id = init.transfer_id;

                // Send init message
                let init_message = Message::ChunkedTransferInit(init);
                network
                    .broadcast(&init_message)
                    .await
                    .map_err(|e| format!("Failed to broadcast chunked transfer init: {}", e))?;

                // Send chunks
                for (i, chunk_data) in content_for_send.data.chunks(chunk_size).enumerate() {
                    let chunk = crate::protocol::ChunkedTransferData::new(
                        transfer_id,
                        i as u32,
                        chunk_data,
                    );
                    let chunk_message = Message::ChunkedTransferData(chunk);

                    network.broadcast(&chunk_message).await.map_err(|e| {
                        format!(
                            "Failed to broadcast chunk {}/{}: {}",
                            i + 1,
                            total_chunks,
                            e
                        )
                    })?;

                    tracing::debug!("Sent chunk {}/{}", i + 1, total_chunks);
                }

                // Send completion message
                let complete = crate::protocol::ChunkedTransferComplete {
                    transfer_id,
                    state: crate::protocol::TransferState::Completed,
                    error: None,
                };
                let complete_message = Message::ChunkedTransferComplete(complete);
                network
                    .broadcast(&complete_message)
                    .await
                    .map_err(|e| format!("Failed to broadcast transfer complete: {}", e))?;

                tracing::info!("Chunked transfer {} completed successfully", transfer_id);
            } else {
                // Use regular single-message transfer for small content
                let update = ClipboardUpdate::new(content_for_send);
                let message = Message::ClipboardUpdate(update);
                network
                    .broadcast(&message)
                    .await
                    .map_err(|e| format!("Failed to broadcast message: {}", e))?;
            }
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
        // Save settings to database for persistence
        core.storage
            .save_settings(&settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;

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

/// Poll for network events (polling-based approach until streams are available).
///
/// This is the primary API for receiving network events in Flutter. Call this function
/// periodically (e.g., in a timer or animation frame callback) to receive events.
///
/// The event receiver is automatically set up when `start_network()` is called.
///
/// # Returns
/// - `Some(TossEvent)` if an event is available
/// - `None` if no event is currently pending or the network is not running
///
/// # Note
/// This uses `try_recv` which is non-blocking. Events include:
/// - Device connection/disconnection
/// - Clipboard updates from remote devices
/// - Network errors
#[frb(sync)]
pub fn poll_event() -> Option<TossEvent> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref()?;

    if let Some(ref receiver_arc) = core.event_receiver {
        // Try to receive an event (non-blocking)
        let mut receiver = receiver_arc
            .lock()
            .expect("event_receiver mutex poisoned - this is a bug");
        match receiver.try_recv() {
            Ok(NetworkEvent::PeerConnected {
                device_id,
                device_name,
                connection_type,
            }) => {
                let conn_type_str = match connection_type {
                    ConnectionType::Direct => "direct",
                    ConnectionType::StunReflexive => "stun_reflexive",
                    ConnectionType::TurnRelay => "turn_relay",
                    ConnectionType::WebSocketRelay => "websocket_relay",
                    ConnectionType::Unknown => "unknown",
                };
                Some(TossEvent::DeviceConnected {
                    device: DeviceInfoDto {
                        id: hex::encode(device_id),
                        name: device_name,
                        is_online: true,
                        last_seen: 0,
                        platform: "unknown".to_string(), // Platform info not available in event yet
                        connection_type: conn_type_str.to_string(),
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
                                                timestamp: current_unix_timestamp_millis(),
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
                                            created_at: current_unix_timestamp_secs(),
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
                            timestamp: current_unix_timestamp_millis(),
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
    get_clipboard_history_filtered(limit, None, None)
}

/// Get clipboard history with optional date range filtering
///
/// # Arguments
/// * `limit` - Optional maximum number of items to return
/// * `start_date_millis` - Optional start date as Unix milliseconds (inclusive)
/// * `end_date_millis` - Optional end date as Unix milliseconds (inclusive)
#[frb(sync)]
pub fn get_clipboard_history_filtered(
    limit: Option<u32>,
    start_date_millis: Option<u64>,
    end_date_millis: Option<u64>,
) -> Vec<ClipboardItemDto> {
    let guard = TOSS_INSTANCE.read();
    let core = match guard.as_ref() {
        Some(c) => c,
        None => return Vec::new(),
    };

    // Convert milliseconds to seconds for database query
    let start_timestamp = start_date_millis.map(|ms| ms / 1000);
    let end_timestamp = end_date_millis.map(|ms| ms / 1000);

    let history_items =
        match core
            .storage
            .history()
            .get_items_by_date_range(start_timestamp, end_timestamp, limit)
        {
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

    tracing::info!("Clipboard history cleared manually");
    Ok(())
}

/// Cleanup old clipboard history based on retention period (history_days setting)
/// Returns the number of items cleaned up
#[frb(sync)]
pub fn cleanup_old_history() -> Result<u32, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let retention_days = core.settings.history_days;
    let deleted_count = core
        .storage
        .cleanup_old_history(retention_days)
        .map_err(|e| format!("Failed to cleanup old history: {}", e))?;

    Ok(deleted_count as u32)
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
                .map(|peer| {
                    let conn_type_str = match peer.connection_type {
                        ConnectionType::Direct => "direct",
                        ConnectionType::StunReflexive => "stun_reflexive",
                        ConnectionType::TurnRelay => "turn_relay",
                        ConnectionType::WebSocketRelay => "websocket_relay",
                        ConnectionType::Unknown => "unknown",
                    };
                    DeviceInfoDto {
                        id: hex::encode(peer.device_id),
                        name: peer.device_name,
                        is_online: peer.is_connected,
                        last_seen: 0,
                        platform: "unknown".to_string(), // Platform info not available in PeerInfo yet
                        connection_type: conn_type_str.to_string(),
                    }
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

/// Sign a message with the device's identity key
/// Returns the signature as a base64-encoded string
/// Used for WebSocket authentication
#[frb(sync)]
pub fn sign_message(message: String) -> Result<String, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let signature = core.identity.sign(message.as_bytes());
    let signature_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature);

    Ok(signature_b64)
}

/// Decrypted clipboard content from history
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipboardContentDto {
    pub content_type: String,
    pub data: Vec<u8>,
}

// ============================================================================
// Streaming / Chunked Transfer API
// ============================================================================

/// Progress information for clipboard transfers
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferProgressDto {
    /// Unique transfer identifier
    pub transfer_id: u64,
    /// Total number of chunks
    pub total_chunks: u32,
    /// Number of chunks completed
    pub chunks_completed: u32,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Progress percentage (0.0 - 1.0)
    pub progress: f64,
    /// Transfer state: "initiated", "in_progress", "completed", "failed", "cancelled"
    pub state: String,
}

impl From<crate::protocol::TransferProgress> for TransferProgressDto {
    fn from(p: crate::protocol::TransferProgress) -> Self {
        let state = match p.state {
            crate::protocol::TransferState::Initiated => "initiated",
            crate::protocol::TransferState::InProgress => "in_progress",
            crate::protocol::TransferState::Completed => "completed",
            crate::protocol::TransferState::Failed => "failed",
            crate::protocol::TransferState::Cancelled => "cancelled",
        };
        Self {
            transfer_id: p.transfer_id,
            total_chunks: p.total_chunks,
            chunks_completed: p.chunks_completed,
            bytes_transferred: p.bytes_transferred,
            total_bytes: p.total_bytes,
            progress: p.percentage(),
            state: state.to_string(),
        }
    }
}

/// Streaming configuration DTO for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamingConfigDto {
    /// Chunk size in bytes (default: 1 MB)
    pub chunk_size: u32,
    /// Threshold for using chunked transfer (default: 1 MB)
    pub chunked_threshold: u32,
    /// Enable streaming for large content
    pub enabled: bool,
}

impl Default for StreamingConfigDto {
    fn default() -> Self {
        Self {
            chunk_size: crate::protocol::DEFAULT_CHUNK_SIZE as u32,
            chunked_threshold: crate::protocol::CHUNKED_TRANSFER_THRESHOLD as u32,
            enabled: true,
        }
    }
}

/// Get current streaming configuration
#[frb(sync)]
pub fn get_streaming_config() -> StreamingConfigDto {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| StreamingConfigDto {
            chunk_size: core.settings.streaming_chunk_size,
            chunked_threshold: crate::protocol::CHUNKED_TRANSFER_THRESHOLD as u32,
            enabled: core.settings.streaming_enabled,
        })
        .unwrap_or_default()
}

/// Update streaming configuration
#[frb(sync)]
pub fn update_streaming_config(config: StreamingConfigDto) -> Result<(), String> {
    // Validate chunk size
    let chunk_size = config.chunk_size as usize;
    if chunk_size < crate::protocol::MIN_CHUNK_SIZE {
        return Err(format!(
            "Chunk size too small (min: {} bytes)",
            crate::protocol::MIN_CHUNK_SIZE
        ));
    }
    if chunk_size > crate::protocol::MAX_CHUNK_SIZE {
        return Err(format!(
            "Chunk size too large (max: {} bytes)",
            crate::protocol::MAX_CHUNK_SIZE
        ));
    }

    if let Some(ref mut core) = *TOSS_INSTANCE.write() {
        core.settings.streaming_chunk_size = config.chunk_size;
        core.settings.streaming_enabled = config.enabled;

        // Save settings to database for persistence
        core.storage
            .save_settings(&core.settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        Ok(())
    } else {
        Err("Toss not initialized".to_string())
    }
}

/// Check if content should use chunked transfer based on size
#[frb(sync)]
pub fn should_use_chunked_transfer(content_size_bytes: u64) -> bool {
    let guard = TOSS_INSTANCE.read();
    if let Some(core) = guard.as_ref() {
        core.settings.streaming_enabled
            && content_size_bytes > crate::protocol::CHUNKED_TRANSFER_THRESHOLD as u64
    } else {
        false
    }
}

/// Get the current chunk size setting in bytes
#[frb(sync)]
pub fn get_chunk_size() -> u32 {
    TOSS_INSTANCE
        .read()
        .as_ref()
        .map(|core| core.settings.streaming_chunk_size)
        .unwrap_or(crate::protocol::DEFAULT_CHUNK_SIZE as u32)
}

/// Calculate the number of chunks needed for a given content size
#[frb(sync)]
pub fn calculate_chunk_count(content_size_bytes: u64) -> u32 {
    let chunk_size = get_chunk_size() as u64;
    if chunk_size == 0 {
        return 1;
    }
    content_size_bytes.div_ceil(chunk_size) as u32
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
        // Streaming settings
        assert!(settings.streaming_enabled);
        assert_eq!(
            settings.streaming_chunk_size,
            crate::protocol::DEFAULT_CHUNK_SIZE as u32
        );
    }

    #[test]
    fn test_streaming_config_dto_default() {
        let config = StreamingConfigDto::default();
        assert!(config.enabled);
        assert_eq!(
            config.chunk_size,
            crate::protocol::DEFAULT_CHUNK_SIZE as u32
        );
        assert_eq!(
            config.chunked_threshold,
            crate::protocol::CHUNKED_TRANSFER_THRESHOLD as u32
        );
    }

    #[test]
    fn test_calculate_chunk_count() {
        // Using default chunk size of 1 MB
        let chunk_size = crate::protocol::DEFAULT_CHUNK_SIZE as u64;

        // Exact multiple
        assert_eq!(((2 * chunk_size + chunk_size - 1) / chunk_size) as u32, 2);

        // With remainder
        assert_eq!(
            ((2 * chunk_size + 100 + chunk_size - 1) / chunk_size) as u32,
            3
        );

        // Less than one chunk
        assert_eq!(((1000 + chunk_size - 1) / chunk_size) as u32, 1);
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
