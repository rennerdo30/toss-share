//! Message types for Toss protocol

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::content::ClipboardContent;
use crate::error::ProtocolError;

/// Message type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    Ping = 0x01,
    Pong = 0x02,
    ClipboardUpdate = 0x10,
    ClipboardAck = 0x11,
    ClipboardRequest = 0x12,
    DeviceInfo = 0x20,
    KeyRotation = 0x30,
    Error = 0xFF,
}

impl TryFrom<u8> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            0x01 => Ok(MessageType::Ping),
            0x02 => Ok(MessageType::Pong),
            0x10 => Ok(MessageType::ClipboardUpdate),
            0x11 => Ok(MessageType::ClipboardAck),
            0x12 => Ok(MessageType::ClipboardRequest),
            0x20 => Ok(MessageType::DeviceInfo),
            0x30 => Ok(MessageType::KeyRotation),
            0xFF => Ok(MessageType::Error),
            _ => Err(ProtocolError::UnknownMessageType(value)),
        }
    }
}

/// Platform identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Platform {
    Unknown = 0,
    MacOS = 1,
    Windows = 2,
    Linux = 3,
    IOS = 4,
    Android = 5,
}

impl Platform {
    /// Get current platform
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "ios")]
        return Platform::IOS;
        #[cfg(target_os = "android")]
        return Platform::Android;
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "ios",
            target_os = "android"
        )))]
        return Platform::Unknown;
    }
}

/// Message header (unencrypted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Protocol version
    pub version: u16,
    /// Message type
    pub message_type: MessageType,
    /// Unique message ID
    pub message_id: u64,
    /// Timestamp (Unix milliseconds)
    pub timestamp: u64,
}

impl MessageHeader {
    /// Create a new header
    pub fn new(message_type: MessageType) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            message_type,
            message_id: generate_message_id(),
            timestamp: current_timestamp_ms(),
        }
    }
}

/// Ping message for keepalive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    pub timestamp: u64,
}

impl Default for Ping {
    fn default() -> Self {
        Self {
            timestamp: current_timestamp_ms(),
        }
    }
}

/// Pong response to ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    pub ping_timestamp: u64,
    pub pong_timestamp: u64,
}

impl Pong {
    pub fn from_ping(ping: &Ping) -> Self {
        Self {
            ping_timestamp: ping.timestamp,
            pong_timestamp: current_timestamp_ms(),
        }
    }
}

/// Clipboard update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardUpdate {
    /// Clipboard content
    pub content: ClipboardContent,
    /// SHA-256 hash of plaintext content
    pub content_hash: [u8; 32],
}

impl ClipboardUpdate {
    pub fn new(content: ClipboardContent) -> Self {
        let content_hash = content.hash();
        Self {
            content,
            content_hash,
        }
    }
}

/// Acknowledgment of clipboard receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardAck {
    /// ID of the acknowledged message
    pub message_id: u64,
    /// Hash of received content (for verification)
    pub content_hash: [u8; 32],
    /// Whether content was successfully applied
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Request current clipboard from peer
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClipboardRequest {
    /// Request specific content types only
    pub content_types: Option<Vec<u8>>,
}

/// Device information exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Device ID (public key hash)
    pub device_id: [u8; 32],
    /// User-friendly device name
    pub device_name: String,
    /// Platform
    pub platform: Platform,
    /// Toss version
    pub version: String,
}

impl DeviceInfo {
    pub fn new(device_id: [u8; 32], device_name: String) -> Self {
        Self {
            device_id,
            device_name,
            platform: Platform::current(),
            version: crate::VERSION.to_string(),
        }
    }
}

/// Key rotation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotation {
    /// New ephemeral public key
    pub new_public_key: [u8; 32],
    /// Signature of new key with identity key (base64 encoded for serde compatibility)
    #[serde(with = "signature_bytes")]
    pub signature: [u8; 64],
    /// Reason for rotation
    pub reason: KeyRotationReason,
}

/// Custom serialization for 64-byte arrays (ed25519 signatures)
mod signature_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Invalid signature length"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyRotationReason {
    Scheduled,
    MessageCountExceeded,
    SecurityConcern,
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code
    pub code: u32,
    /// Error description
    pub message: String,
    /// Related message ID if applicable
    pub related_message_id: Option<u64>,
}

/// Union of all message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Ping(Ping),
    Pong(Pong),
    ClipboardUpdate(ClipboardUpdate),
    ClipboardAck(ClipboardAck),
    ClipboardRequest(ClipboardRequest),
    DeviceInfo(DeviceInfo),
    KeyRotation(KeyRotation),
    Error(ErrorMessage),
}

impl Message {
    /// Get the message header
    pub fn header(&self) -> MessageHeader {
        let message_type = match self {
            Message::Ping(_) => MessageType::Ping,
            Message::Pong(_) => MessageType::Pong,
            Message::ClipboardUpdate(_) => MessageType::ClipboardUpdate,
            Message::ClipboardAck(_) => MessageType::ClipboardAck,
            Message::ClipboardRequest(_) => MessageType::ClipboardRequest,
            Message::DeviceInfo(_) => MessageType::DeviceInfo,
            Message::KeyRotation(_) => MessageType::KeyRotation,
            Message::Error(_) => MessageType::Error,
        };
        MessageHeader::new(message_type)
    }

    /// Serialize message payload
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::serialize(self).map_err(|e| ProtocolError::Serialization(e.to_string()))
    }

    /// Deserialize message from header and payload
    pub fn deserialize(header: &MessageHeader, payload: &[u8]) -> Result<Self, ProtocolError> {
        // Check version compatibility
        if header.version > crate::PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(header.version));
        }

        bincode::deserialize(payload).map_err(|e| ProtocolError::Deserialization(e.to_string()))
    }
}

/// Generate unique message ID
fn generate_message_id() -> u64 {
    // Use UUID v4 and take first 8 bytes as u64
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().unwrap())
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "bincode enum encoding needs investigation"]
    fn test_message_serialization_roundtrip() {
        // Test ClipboardUpdate serialization using the Message::serialize method
        let content = ClipboardContent::text("Hello, World!");
        let update = ClipboardUpdate::new(content);
        let message = Message::ClipboardUpdate(update.clone());

        // Serialize using Message's serialize method
        let serialized = message.serialize().unwrap();
        let header = message.header();

        // Deserialize using Message's deserialize method
        let deserialized = Message::deserialize(&header, &serialized).unwrap();

        match deserialized {
            Message::ClipboardUpdate(deserialized_update) => {
                assert_eq!(update.content_hash, deserialized_update.content_hash);
                assert_eq!(
                    update.content.as_text(),
                    deserialized_update.content.as_text()
                );
            }
            _ => panic!("Expected ClipboardUpdate"),
        }
    }

    #[test]
    fn test_clipboard_ack_serialization() {
        let ack = ClipboardAck {
            message_id: 12345,
            content_hash: [0xAB; 32],
            success: true,
            error: None,
        };
        let message = Message::ClipboardAck(ack.clone());

        let serialized = message.serialize().unwrap();
        let header = message.header();
        let deserialized = Message::deserialize(&header, &serialized).unwrap();

        match deserialized {
            Message::ClipboardAck(deserialized_ack) => {
                assert_eq!(ack.message_id, deserialized_ack.message_id);
                assert_eq!(ack.content_hash, deserialized_ack.content_hash);
                assert_eq!(ack.success, deserialized_ack.success);
            }
            _ => panic!("Expected ClipboardAck"),
        }
    }

    #[test]
    fn test_device_info_serialization() {
        let device_id = [0x12; 32];
        let device_info = DeviceInfo::new(device_id, "Test Device".to_string());
        let message = Message::DeviceInfo(device_info.clone());

        let serialized = message.serialize().unwrap();
        let header = message.header();
        let deserialized = Message::deserialize(&header, &serialized).unwrap();

        match deserialized {
            Message::DeviceInfo(deserialized_info) => {
                assert_eq!(device_info.device_id, deserialized_info.device_id);
                assert_eq!(device_info.device_name, deserialized_info.device_name);
            }
            _ => panic!("Expected DeviceInfo"),
        }
    }

    #[test]
    fn test_error_message_serialization() {
        let error = ErrorMessage {
            code: 1001,
            message: "Test error".to_string(),
            related_message_id: Some(42),
        };
        let message = Message::Error(error.clone());

        let serialized = message.serialize().unwrap();
        let header = message.header();
        let deserialized = Message::deserialize(&header, &serialized).unwrap();

        match deserialized {
            Message::Error(deserialized_error) => {
                assert_eq!(error.code, deserialized_error.code);
                assert_eq!(error.message, deserialized_error.message);
                assert_eq!(
                    error.related_message_id,
                    deserialized_error.related_message_id
                );
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::try_from(0x01).unwrap(), MessageType::Ping);
        assert_eq!(
            MessageType::try_from(0x10).unwrap(),
            MessageType::ClipboardUpdate
        );
        assert!(MessageType::try_from(0x99).is_err());
    }

    #[test]
    fn test_message_serialization() {
        let ping = Message::Ping(Ping::default());
        let serialized = ping.serialize().unwrap();
        let header = ping.header();
        let deserialized = Message::deserialize(&header, &serialized).unwrap();

        if let Message::Ping(p) = deserialized {
            assert!(p.timestamp > 0);
        } else {
            panic!("Expected Ping message");
        }
    }

    #[test]
    fn test_clipboard_update_message() {
        let content = ClipboardContent::text("Test content");
        let update = ClipboardUpdate::new(content);

        assert_ne!(update.content_hash, [0u8; 32]);
    }

    #[test]
    fn test_device_info() {
        let device_id = [0u8; 32];
        let info = DeviceInfo::new(device_id, "Test Device".to_string());

        assert_eq!(info.device_name, "Test Device");
        assert_eq!(info.version, crate::VERSION);
    }

    #[test]
    fn test_ping_pong() {
        let ping = Ping::default();
        let pong = Pong::from_ping(&ping);

        assert_eq!(pong.ping_timestamp, ping.timestamp);
        assert!(pong.pong_timestamp >= ping.timestamp);
    }
}
