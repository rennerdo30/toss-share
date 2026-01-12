//! Error types for Toss

use thiserror::Error;

/// Main error type for Toss operations
#[derive(Error, Debug)]
pub enum TossError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Clipboard error: {0}")]
    Clipboard(#[from] ClipboardError),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Cryptographic operation errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),

    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Decryption failed: {0}")]
    Decryption(String),

    #[error("Invalid key format")]
    InvalidKey,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Signature verification failed")]
    SignatureVerification,

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Pairing failed: {0}")]
    PairingFailed(String),

    #[error("Session expired")]
    SessionExpired,
}

/// Network operation errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Connection timeout")]
    Timeout,

    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Relay error: {0}")]
    Relay(String),

    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Address parse error: {0}")]
    AddressParse(String),

    #[error("TLS error: {0}")]
    Tls(String),
}

/// Protocol/message errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u16),

    #[error("Unknown message type: {0}")]
    UnknownMessageType(u8),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Message too large: {0} bytes (max: {1})")]
    MessageTooLarge(usize, usize),

    #[error("Invalid content type")]
    InvalidContentType,
}

/// Clipboard operation errors
#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard access denied")]
    AccessDenied,

    #[error("Clipboard is empty")]
    Empty,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Clipboard operation failed: {0}")]
    OperationFailed(String),

    #[error("Image conversion error: {0}")]
    ImageConversion(String),
}

impl From<bincode::Error> for ProtocolError {
    fn from(e: bincode::Error) -> Self {
        ProtocolError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for ProtocolError {
    fn from(e: serde_json::Error) -> Self {
        ProtocolError::Serialization(e.to_string())
    }
}
