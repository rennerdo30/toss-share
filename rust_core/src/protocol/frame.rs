//! Wire frame encoding with encryption and optional compression

use super::compression::{
    compress_payload, decompress_payload, CompressionConfig, CompressionStats,
};
use super::message::MessageHeader;
use crate::crypto::{decrypt, encrypt, EncryptedMessage, KEY_SIZE, NONCE_SIZE, TAG_SIZE};
use crate::error::{CryptoError, ProtocolError};

/// Frame format:
/// [version: 2 bytes][type: 1 byte][reserved: 1 byte][message_id: 8 bytes][timestamp: 8 bytes][payload_length: 4 bytes][nonce: 12 bytes][encrypted_payload: N bytes][tag: 16 bytes]
const HEADER_SIZE: usize = 2 + 1 + 1 + 8 + 8 + 4; // 24 bytes

/// Wire frame containing encrypted message
#[derive(Debug, Clone)]
pub struct Frame {
    /// Message header (unencrypted, for routing)
    pub header: MessageHeader,
    /// Encrypted payload
    pub encrypted: EncryptedMessage,
}

impl Frame {
    /// Create a new frame by encrypting a message
    pub fn encrypt(
        header: &MessageHeader,
        payload: &[u8],
        key: &[u8; KEY_SIZE],
    ) -> Result<Self, CryptoError> {
        // AAD is the serialized header for authentication
        let aad = Self::header_to_bytes(header);
        let encrypted = encrypt(key, payload, &aad)?;

        Ok(Self {
            header: header.clone(),
            encrypted,
        })
    }

    /// Decrypt the frame payload
    pub fn decrypt(&self, key: &[u8; KEY_SIZE]) -> Result<(MessageHeader, Vec<u8>), CryptoError> {
        let aad = Self::header_to_bytes(&self.header);
        let payload = decrypt(key, &self.encrypted, &aad)?;
        Ok((self.header.clone(), payload))
    }

    /// Serialize frame to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let encrypted_bytes = self.encrypted.to_bytes();

        let payload_len = encrypted_bytes.len() as u32;
        let total_size = HEADER_SIZE + encrypted_bytes.len();

        let mut bytes = Vec::with_capacity(total_size);

        // Header
        bytes.extend_from_slice(&self.header.version.to_le_bytes());
        bytes.push(self.header.message_type as u8);
        bytes.push(0); // Reserved
        bytes.extend_from_slice(&self.header.message_id.to_le_bytes());
        bytes.extend_from_slice(&self.header.timestamp.to_le_bytes());
        bytes.extend_from_slice(&payload_len.to_le_bytes());

        // Encrypted payload (nonce + ciphertext + tag)
        bytes.extend_from_slice(&encrypted_bytes);

        bytes
    }

    /// Parse frame from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < HEADER_SIZE + NONCE_SIZE + TAG_SIZE {
            return Err(ProtocolError::InvalidFormat("Frame too short".to_string()));
        }

        // Parse header
        let version = u16::from_le_bytes([bytes[0], bytes[1]]);
        let message_type = bytes[2].try_into()?;
        // bytes[3] is reserved
        let message_id = u64::from_le_bytes(
            bytes[4..12]
                .try_into()
                .expect("slice length verified above"),
        );
        let timestamp = u64::from_le_bytes(
            bytes[12..20]
                .try_into()
                .expect("slice length verified above"),
        );
        let payload_len = u32::from_le_bytes(
            bytes[20..24]
                .try_into()
                .expect("slice length verified above"),
        ) as usize;

        // Validate payload length
        if bytes.len() < HEADER_SIZE + payload_len {
            return Err(ProtocolError::InvalidFormat(
                "Payload length mismatch".to_string(),
            ));
        }

        if payload_len > super::MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge(
                payload_len,
                super::MAX_MESSAGE_SIZE,
            ));
        }

        // Parse encrypted payload
        let encrypted_bytes = &bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
        let encrypted = EncryptedMessage::from_bytes(encrypted_bytes).map_err(|e| {
            ProtocolError::InvalidFormat(format!("Invalid encrypted message: {}", e))
        })?;

        let header = MessageHeader {
            version,
            message_type,
            message_id,
            timestamp,
        };

        Ok(Self { header, encrypted })
    }

    /// Serialize header to bytes (used as AAD)
    fn header_to_bytes(header: &MessageHeader) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);
        bytes.extend_from_slice(&header.version.to_le_bytes());
        bytes.push(header.message_type as u8);
        bytes.push(0); // Reserved
        bytes.extend_from_slice(&header.message_id.to_le_bytes());
        bytes.extend_from_slice(&header.timestamp.to_le_bytes());
        bytes
    }

    /// Get the unencrypted header for routing decisions
    pub fn peek_header(bytes: &[u8]) -> Result<MessageHeader, ProtocolError> {
        if bytes.len() < HEADER_SIZE {
            return Err(ProtocolError::InvalidFormat(
                "Frame too short for header".to_string(),
            ));
        }

        let version = u16::from_le_bytes([bytes[0], bytes[1]]);
        let message_type = bytes[2].try_into()?;
        let message_id = u64::from_le_bytes(
            bytes[4..12]
                .try_into()
                .expect("slice length verified above"),
        );
        let timestamp = u64::from_le_bytes(
            bytes[12..20]
                .try_into()
                .expect("slice length verified above"),
        );

        Ok(MessageHeader {
            version,
            message_type,
            message_id,
            timestamp,
        })
    }
}

/// Result of a compressed frame operation, including statistics
#[derive(Debug, Clone)]
pub struct CompressedFrameResult {
    /// The encrypted frame
    pub frame: Frame,
    /// Compression statistics
    pub compression_stats: CompressionStats,
}

/// Wrapper for Frame that supports transparent compression
///
/// CompressedFrame provides the same interface as Frame but automatically
/// compresses payloads above the configured threshold before encryption.
/// This is fully backwards compatible - receivers can handle both compressed
/// and uncompressed payloads transparently.
#[derive(Debug, Clone)]
pub struct CompressedFrame {
    config: CompressionConfig,
}

impl Default for CompressedFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressedFrame {
    /// Create a new CompressedFrame handler with default configuration
    pub fn new() -> Self {
        Self {
            config: CompressionConfig::default(),
        }
    }

    /// Create a new CompressedFrame handler with custom configuration
    pub fn with_config(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Get the current compression configuration
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }

    /// Create a frame by optionally compressing then encrypting the payload
    ///
    /// Compression is applied transparently if the payload exceeds the threshold.
    /// The compression magic bytes are prepended to compressed payloads, allowing
    /// the receiver to detect and decompress automatically.
    pub fn encrypt(
        &self,
        header: &MessageHeader,
        payload: &[u8],
        key: &[u8; KEY_SIZE],
    ) -> Result<CompressedFrameResult, ProtocolError> {
        // Compress the payload if it exceeds threshold
        let (compressed_payload, compression_stats) = compress_payload(payload, &self.config)?;

        // Create the encrypted frame with the (possibly compressed) payload
        let frame = Frame::encrypt(header, &compressed_payload, key)
            .map_err(|e| ProtocolError::Serialization(format!("Encryption failed: {}", e)))?;

        Ok(CompressedFrameResult {
            frame,
            compression_stats,
        })
    }

    /// Decrypt and decompress a frame payload
    ///
    /// Automatically detects whether the payload was compressed and decompresses
    /// if necessary. This maintains backwards compatibility with uncompressed payloads.
    pub fn decrypt(
        frame: &Frame,
        key: &[u8; KEY_SIZE],
    ) -> Result<(MessageHeader, Vec<u8>, CompressionStats), ProtocolError> {
        // Decrypt the frame
        let (header, encrypted_payload) = frame
            .decrypt(key)
            .map_err(|e| ProtocolError::Deserialization(format!("Decryption failed: {}", e)))?;

        // Decompress if needed (auto-detected by magic bytes)
        let (payload, compression_stats) = decompress_payload(&encrypted_payload)?;

        Ok((header, payload, compression_stats))
    }

    /// Convenience method to encrypt without compression (for testing or small payloads)
    pub fn encrypt_uncompressed(
        header: &MessageHeader,
        payload: &[u8],
        key: &[u8; KEY_SIZE],
    ) -> Result<Frame, CryptoError> {
        Frame::encrypt(header, payload, key)
    }
}

/// Transfer statistics including compression information
#[derive(Debug, Clone, Default)]
pub struct TransferStats {
    /// Total bytes sent over the wire (after compression and encryption)
    pub bytes_sent: usize,
    /// Total bytes received over the wire
    pub bytes_received: usize,
    /// Original payload size before compression
    pub original_payload_size: usize,
    /// Compressed payload size (same as original if not compressed)
    pub compressed_payload_size: usize,
    /// Whether compression was used
    pub was_compressed: bool,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
}

impl TransferStats {
    /// Update stats with compression information
    pub fn record_send(
        &mut self,
        original_size: usize,
        compressed_size: usize,
        was_compressed: bool,
    ) {
        self.bytes_sent += compressed_size;
        self.original_payload_size += original_size;
        self.compressed_payload_size += compressed_size;
        self.was_compressed = self.was_compressed || was_compressed;
        self.messages_sent += 1;
    }

    /// Update stats for received data
    pub fn record_receive(&mut self, size: usize) {
        self.bytes_received += size;
        self.messages_received += 1;
    }

    /// Calculate overall compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.original_payload_size == 0 {
            1.0
        } else {
            self.compressed_payload_size as f64 / self.original_payload_size as f64
        }
    }

    /// Calculate bandwidth saved in bytes
    pub fn bandwidth_saved(&self) -> usize {
        self.original_payload_size
            .saturating_sub(self.compressed_payload_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::MessageType;
    use rand::{rngs::StdRng, RngCore, SeedableRng};

    fn random_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        StdRng::from_entropy().fill_bytes(&mut key);
        key
    }

    #[test]
    fn test_frame_roundtrip() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::Ping);
        let payload = b"Hello, World!";

        let frame = Frame::encrypt(&header, payload, &key).unwrap();
        let bytes = frame.to_bytes();

        let parsed = Frame::from_bytes(&bytes).unwrap();
        let (parsed_header, decrypted) = parsed.decrypt(&key).unwrap();

        assert_eq!(parsed_header.version, header.version);
        assert_eq!(parsed_header.message_type, header.message_type);
        assert_eq!(parsed_header.message_id, header.message_id);
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn test_peek_header() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::ClipboardUpdate);
        let payload = b"test";

        let frame = Frame::encrypt(&header, payload, &key).unwrap();
        let bytes = frame.to_bytes();

        let peeked = Frame::peek_header(&bytes).unwrap();
        assert_eq!(peeked.message_type, MessageType::ClipboardUpdate);
        assert_eq!(peeked.message_id, header.message_id);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = random_key();
        let key2 = random_key();
        let header = MessageHeader::new(MessageType::Ping);
        let payload = b"Secret";

        let frame = Frame::encrypt(&header, payload, &key1).unwrap();
        let bytes = frame.to_bytes();

        let parsed = Frame::from_bytes(&bytes).unwrap();
        let result = parsed.decrypt(&key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_header_fails() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::Ping);
        let payload = b"Message";

        let frame = Frame::encrypt(&header, payload, &key).unwrap();
        let mut bytes = frame.to_bytes();

        // Tamper with the message type in header
        bytes[2] = 0x10; // Change from Ping to ClipboardUpdate

        let parsed = Frame::from_bytes(&bytes).unwrap();
        let result = parsed.decrypt(&key);

        // Decryption should fail because AAD (header) was tampered
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_too_short() {
        let result = Frame::from_bytes(&[0; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compressed_frame_small_payload() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::Ping);
        let payload = b"Small payload"; // Below compression threshold

        let compressed_frame = CompressedFrame::new();
        let result = compressed_frame.encrypt(&header, payload, &key).unwrap();

        // Small payload should not be compressed
        assert!(!result.compression_stats.was_compressed);

        // Decrypt and verify
        let (parsed_header, decrypted, stats) =
            CompressedFrame::decrypt(&result.frame, &key).unwrap();

        assert_eq!(parsed_header.message_id, header.message_id);
        assert_eq!(decrypted, payload);
        assert!(!stats.was_compressed);
    }

    #[test]
    fn test_compressed_frame_large_payload() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::ClipboardUpdate);
        // Create compressible payload above threshold
        let payload = b"This is a test message that should be compressed! ".repeat(500);

        let config = CompressionConfig::with_threshold(1024); // 1KB threshold
        let compressed_frame = CompressedFrame::with_config(config);
        let result = compressed_frame.encrypt(&header, &payload, &key).unwrap();

        // Large payload should be compressed
        assert!(result.compression_stats.was_compressed);
        assert!(result.compression_stats.compressed_size < result.compression_stats.original_size);

        // Decrypt and verify
        let (parsed_header, decrypted, stats) =
            CompressedFrame::decrypt(&result.frame, &key).unwrap();

        assert_eq!(parsed_header.message_id, header.message_id);
        assert_eq!(decrypted, payload);
        assert!(stats.was_compressed);
    }

    #[test]
    fn test_compressed_frame_backwards_compatibility() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::ClipboardUpdate);
        let payload = b"Uncompressed payload for backwards compatibility";

        // Create frame without compression (simulating old client)
        let frame = Frame::encrypt(&header, payload, &key).unwrap();

        // New client should still be able to decrypt uncompressed frame
        let (parsed_header, decrypted, stats) = CompressedFrame::decrypt(&frame, &key).unwrap();

        assert_eq!(parsed_header.message_id, header.message_id);
        assert_eq!(decrypted, payload);
        assert!(!stats.was_compressed);
    }

    #[test]
    fn test_compressed_frame_disabled_compression() {
        let key = random_key();
        let header = MessageHeader::new(MessageType::ClipboardUpdate);
        let payload = b"Test payload ".repeat(1000); // Large payload

        let config = CompressionConfig::disabled();
        let compressed_frame = CompressedFrame::with_config(config);
        let result = compressed_frame.encrypt(&header, &payload, &key).unwrap();

        // Compression should be disabled
        assert!(!result.compression_stats.was_compressed);

        // Decrypt and verify
        let (_, decrypted, _) = CompressedFrame::decrypt(&result.frame, &key).unwrap();
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn test_transfer_stats() {
        let mut stats = TransferStats::default();

        stats.record_send(10000, 2500, true);
        stats.record_send(5000, 5000, false);
        stats.record_receive(2500);
        stats.record_receive(5000);

        assert_eq!(stats.bytes_sent, 7500);
        assert_eq!(stats.bytes_received, 7500);
        assert_eq!(stats.original_payload_size, 15000);
        assert_eq!(stats.compressed_payload_size, 7500);
        assert!(stats.was_compressed);
        assert_eq!(stats.messages_sent, 2);
        assert_eq!(stats.messages_received, 2);
        assert!((stats.compression_ratio() - 0.5).abs() < 0.001);
        assert_eq!(stats.bandwidth_saved(), 7500);
    }
}
