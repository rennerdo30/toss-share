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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toss_error_display() {
        let err = TossError::Crypto(CryptoError::KeyGeneration("test".to_string()));
        assert!(err.to_string().contains("Crypto error"));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_crypto_error_variants() {
        let key_gen = CryptoError::KeyGeneration("failed".to_string());
        assert!(key_gen.to_string().contains("Key generation failed"));

        let encryption = CryptoError::Encryption("aes error".to_string());
        assert!(encryption.to_string().contains("Encryption failed"));

        let decryption = CryptoError::Decryption("decrypt error".to_string());
        assert!(decryption.to_string().contains("Decryption failed"));

        let invalid_key = CryptoError::InvalidKey;
        assert!(invalid_key.to_string().contains("Invalid key format"));

        let invalid_sig = CryptoError::InvalidSignature;
        assert!(invalid_sig.to_string().contains("Invalid signature"));
    }

    #[test]
    fn test_network_error_variants() {
        let conn_failed = NetworkError::ConnectionFailed("timeout".to_string());
        assert!(conn_failed.to_string().contains("Connection failed"));

        let timeout = NetworkError::Timeout;
        assert!(timeout.to_string().contains("Connection timeout"));

        let not_auth = NetworkError::NotAuthenticated;
        assert!(not_auth.to_string().contains("Not authenticated"));

        let peer_not_found = NetworkError::PeerNotFound("device-123".to_string());
        assert!(peer_not_found.to_string().contains("Peer not found"));
    }

    #[test]
    fn test_protocol_error_variants() {
        let invalid_format = ProtocolError::InvalidFormat("bad format".to_string());
        assert!(invalid_format.to_string().contains("Invalid message format"));

        let unsupported = ProtocolError::UnsupportedVersion(99);
        assert!(unsupported.to_string().contains("Unsupported version"));

        let too_large = ProtocolError::MessageTooLarge(10000, 5000);
        assert!(too_large.to_string().contains("Message too large"));
        assert!(too_large.to_string().contains("10000"));
        assert!(too_large.to_string().contains("5000"));
    }

    #[test]
    fn test_clipboard_error_variants() {
        let access_denied = ClipboardError::AccessDenied;
        assert!(access_denied.to_string().contains("Clipboard access denied"));

        let empty = ClipboardError::Empty;
        assert!(empty.to_string().contains("Clipboard is empty"));

        let unsupported = ClipboardError::UnsupportedFormat("custom".to_string());
        assert!(unsupported.to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_error_conversions() {
        // Test that errors can be converted through the chain
        let crypto_err = CryptoError::KeyGeneration("test".to_string());
        let toss_err: TossError = crypto_err.into();
        assert!(matches!(toss_err, TossError::Crypto(_)));

        let network_err = NetworkError::ConnectionFailed("test".to_string());
        let toss_err: TossError = network_err.into();
        assert!(matches!(toss_err, TossError::Network(_)));
    }
}
