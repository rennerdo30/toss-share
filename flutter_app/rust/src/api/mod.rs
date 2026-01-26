//! FFI API wrapper for flutter_rust_bridge
//!
//! This module provides thin wrapper functions that forward calls to toss_core.
//! flutter_rust_bridge needs to parse source files with #[frb] attributes directly,
//! so we can't just re-export from toss_core.

use flutter_rust_bridge::frb;

// ============================================================================
// Type Definitions (mirrored from toss_core for flutter_rust_bridge visibility)
// ============================================================================

/// Toss settings
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
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

impl From<toss_core::api::TossSettings> for TossSettings {
    fn from(s: toss_core::api::TossSettings) -> Self {
        Self {
            auto_sync: s.auto_sync,
            sync_text: s.sync_text,
            sync_rich_text: s.sync_rich_text,
            sync_images: s.sync_images,
            sync_files: s.sync_files,
            max_file_size_mb: s.max_file_size_mb,
            history_enabled: s.history_enabled,
            history_days: s.history_days,
            relay_url: s.relay_url,
        }
    }
}

impl From<TossSettings> for toss_core::api::TossSettings {
    fn from(s: TossSettings) -> Self {
        Self {
            auto_sync: s.auto_sync,
            sync_text: s.sync_text,
            sync_rich_text: s.sync_rich_text,
            sync_images: s.sync_images,
            sync_files: s.sync_files,
            max_file_size_mb: s.max_file_size_mb,
            history_enabled: s.history_enabled,
            history_days: s.history_days,
            relay_url: s.relay_url,
        }
    }
}

/// Device information
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub struct DeviceInfoDto {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub last_seen: u64,
    pub platform: String,
}

impl From<toss_core::api::DeviceInfoDto> for DeviceInfoDto {
    fn from(d: toss_core::api::DeviceInfoDto) -> Self {
        Self {
            id: d.id,
            name: d.name,
            is_online: d.is_online,
            last_seen: d.last_seen,
            platform: d.platform,
        }
    }
}

/// Clipboard item for display
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub struct ClipboardItemDto {
    pub id: String,
    pub content_type: String,
    pub preview: String,
    pub size_bytes: u64,
    pub timestamp: u64,
    pub source_device: Option<String>,
}

impl From<toss_core::api::ClipboardItemDto> for ClipboardItemDto {
    fn from(c: toss_core::api::ClipboardItemDto) -> Self {
        Self {
            id: c.id,
            content_type: c.content_type,
            preview: c.preview,
            size_bytes: c.size_bytes,
            timestamp: c.timestamp,
            source_device: c.source_device,
        }
    }
}

/// Pairing info for Flutter
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub struct PairingInfoDto {
    pub code: String,
    pub qr_data: String,
    pub expires_at: u64,
    pub public_key: String,
}

impl From<toss_core::api::PairingInfoDto> for PairingInfoDto {
    fn from(p: toss_core::api::PairingInfoDto) -> Self {
        Self {
            code: p.code,
            qr_data: p.qr_data,
            expires_at: p.expires_at,
            public_key: p.public_key,
        }
    }
}

/// Pairing device info from find_pairing_device
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub struct PairingDeviceDto {
    pub code: String,
    pub public_key: String,
    pub device_name: String,
    pub via_relay: bool,
}

impl From<toss_core::api::PairingDeviceDto> for PairingDeviceDto {
    fn from(p: toss_core::api::PairingDeviceDto) -> Self {
        Self {
            code: p.code,
            public_key: p.public_key,
            device_name: p.device_name,
            via_relay: p.via_relay,
        }
    }
}

/// Result of pairing advertisement registration
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
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

impl From<toss_core::api::AdvertisementResultDto> for AdvertisementResultDto {
    fn from(r: toss_core::api::AdvertisementResultDto) -> Self {
        Self {
            mdns_registered: r.mdns_registered,
            relay_registered: r.relay_registered,
            mdns_error: r.mdns_error,
            relay_error: r.relay_error,
        }
    }
}

/// Decrypted clipboard content from history
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub struct ClipboardContentDto {
    pub content_type: String,
    pub data: Vec<u8>,
}

impl From<toss_core::api::ClipboardContentDto> for ClipboardContentDto {
    fn from(c: toss_core::api::ClipboardContentDto) -> Self {
        Self {
            content_type: c.content_type,
            data: c.data,
        }
    }
}

/// Event types for Flutter
#[derive(Debug, Clone)]
#[frb(dart_metadata=("freezed"))]
pub enum TossEvent {
    ClipboardReceived { item: ClipboardItemDto },
    DeviceConnected { device: DeviceInfoDto },
    DeviceDisconnected { device_id: String },
    PairingRequest { device: DeviceInfoDto },
    Error { message: String },
}

impl From<toss_core::api::TossEvent> for TossEvent {
    fn from(e: toss_core::api::TossEvent) -> Self {
        match e {
            toss_core::api::TossEvent::ClipboardReceived { item } => {
                TossEvent::ClipboardReceived { item: item.into() }
            }
            toss_core::api::TossEvent::DeviceConnected { device } => TossEvent::DeviceConnected {
                device: device.into(),
            },
            toss_core::api::TossEvent::DeviceDisconnected { device_id } => {
                TossEvent::DeviceDisconnected { device_id }
            }
            toss_core::api::TossEvent::PairingRequest { device } => TossEvent::PairingRequest {
                device: device.into(),
            },
            toss_core::api::TossEvent::Error { message } => TossEvent::Error { message },
        }
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize Toss core
#[frb(sync)]
pub fn init_toss(data_dir: String, device_name: String) -> Result<(), String> {
    toss_core::api::init_toss(data_dir, device_name)
}

/// Shutdown Toss
#[frb]
pub async fn shutdown_toss() {
    toss_core::api::shutdown_toss().await
}

// ============================================================================
// Device Identity
// ============================================================================

/// Get device ID
#[frb(sync)]
pub fn get_device_id() -> String {
    toss_core::api::get_device_id()
}

/// Get device name
#[frb(sync)]
pub fn get_device_name() -> String {
    toss_core::api::get_device_name()
}

/// Set device name
#[frb(sync)]
pub fn set_device_name(name: String) -> Result<(), String> {
    toss_core::api::set_device_name(name)
}

// ============================================================================
// Pairing
// ============================================================================

/// Start a new pairing session
#[frb(sync)]
pub fn start_pairing() -> Result<PairingInfoDto, String> {
    toss_core::api::start_pairing().map(|p| p.into())
}

/// Complete pairing with QR data
#[frb(sync)]
pub fn complete_pairing_qr(qr_data: String) -> Result<DeviceInfoDto, String> {
    toss_core::api::complete_pairing_qr(qr_data).map(|d| d.into())
}

/// Complete pairing with manual code
#[frb(sync)]
pub fn complete_pairing_code(
    code: String,
    peer_public_key: Vec<u8>,
) -> Result<DeviceInfoDto, String> {
    toss_core::api::complete_pairing_code(code, peer_public_key).map(|d| d.into())
}

/// Cancel active pairing session
#[frb(sync)]
pub fn cancel_pairing() {
    toss_core::api::cancel_pairing()
}

/// Find a device by pairing code (searches mDNS and relay server)
#[frb]
pub async fn find_pairing_device(code: String) -> Result<PairingDeviceDto, String> {
    toss_core::api::find_pairing_device(code)
        .await
        .map(|p| p.into())
}

/// Complete pairing with a device found via find_pairing_device
#[frb(sync)]
pub fn complete_manual_pairing(
    peer_public_key: String,
    peer_device_name: String,
) -> Result<DeviceInfoDto, String> {
    toss_core::api::complete_manual_pairing(peer_public_key, peer_device_name).map(|d| d.into())
}

/// Register pairing code on relay server and via mDNS for discovery
/// Returns the result indicating which methods succeeded/failed
#[frb]
pub async fn register_pairing_advertisement() -> Result<AdvertisementResultDto, String> {
    toss_core::api::register_pairing_advertisement()
        .await
        .map(|r| r.into())
}

// ============================================================================
// Device Management
// ============================================================================

/// Get list of paired devices
#[frb(sync)]
pub fn get_paired_devices() -> Vec<DeviceInfoDto> {
    toss_core::api::get_paired_devices()
        .into_iter()
        .map(|d| d.into())
        .collect()
}

/// Remove a paired device
#[frb(sync)]
pub fn remove_device(device_id: String) -> Result<(), String> {
    toss_core::api::remove_device(device_id)
}

/// Rename a paired device
#[frb(sync)]
pub fn rename_device(device_id: String, new_name: String) -> Result<(), String> {
    toss_core::api::rename_device(device_id, new_name)
}

// ============================================================================
// Clipboard Operations
// ============================================================================

/// Get current clipboard content
#[frb(sync)]
pub fn get_current_clipboard() -> Option<ClipboardItemDto> {
    toss_core::api::get_current_clipboard().map(|c| c.into())
}

/// Send current clipboard to all devices
#[frb]
pub async fn send_clipboard() -> Result<(), String> {
    toss_core::api::send_clipboard().await
}

/// Send text to all devices
#[frb]
pub async fn send_text(text: String) -> Result<(), String> {
    toss_core::api::send_text(text).await
}

/// Check if clipboard has changed since last check
#[frb(sync)]
pub fn check_clipboard_changed() -> bool {
    toss_core::api::check_clipboard_changed()
}

// ============================================================================
// Settings
// ============================================================================

/// Get current settings
#[frb(sync)]
pub fn get_settings() -> TossSettings {
    toss_core::api::get_settings().into()
}

/// Update settings
#[frb(sync)]
pub fn update_settings(settings: TossSettings) -> Result<(), String> {
    toss_core::api::update_settings(settings.into())
}

// ============================================================================
// Network
// ============================================================================

/// Start networking
#[frb]
pub async fn start_network() -> Result<(), String> {
    toss_core::api::start_network().await
}

/// Stop networking
#[frb]
pub async fn stop_network() {
    toss_core::api::stop_network().await
}

/// Start listening to network events
#[frb]
pub async fn start_event_listener() -> Result<(), String> {
    toss_core::api::start_event_listener().await
}

/// Poll for network events (polling-based approach until streams are available)
#[frb(sync)]
pub fn poll_event() -> Option<TossEvent> {
    toss_core::api::poll_event().map(|e| e.into())
}

/// Get connected devices
#[frb(sync)]
pub fn get_connected_devices() -> Vec<DeviceInfoDto> {
    toss_core::api::get_connected_devices()
        .into_iter()
        .map(|d| d.into())
        .collect()
}

// ============================================================================
// History
// ============================================================================

/// Get clipboard history
#[frb(sync)]
pub fn get_clipboard_history(limit: Option<u32>) -> Vec<ClipboardItemDto> {
    toss_core::api::get_clipboard_history(limit)
        .into_iter()
        .map(|c| c.into())
        .collect()
}

/// Remove clipboard history item
#[frb(sync)]
pub fn remove_history_item(item_id: String) -> Result<(), String> {
    toss_core::api::remove_history_item(item_id)
}

/// Clear clipboard history
#[frb(sync)]
pub fn clear_clipboard_history() -> Result<(), String> {
    toss_core::api::clear_clipboard_history()
}

/// Get decrypted clipboard content from history item
#[frb(sync)]
pub fn get_clipboard_history_content(item_id: String) -> Result<ClipboardContentDto, String> {
    toss_core::api::get_clipboard_history_content(item_id).map(|c| c.into())
}

/// Decrypt and retrieve session key for a paired device
#[frb(sync)]
pub fn get_device_session_key(device_id: String) -> Result<Vec<u8>, String> {
    toss_core::api::get_device_session_key(device_id)
}
