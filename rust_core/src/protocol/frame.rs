//! Wire frame encoding with encryption

use crate::crypto::{decrypt, encrypt, EncryptedMessage, KEY_SIZE, NONCE_SIZE, TAG_SIZE};
use crate::error::{CryptoError, ProtocolError};
use super::message::MessageHeader;

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
        let message_id = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        let timestamp = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
        let payload_len = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;

        // Validate payload length
        if bytes.len() < HEADER_SIZE + payload_len {
            return Err(ProtocolError::InvalidFormat("Payload length mismatch".to_string()));
        }

        if payload_len > super::MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge(payload_len, super::MAX_MESSAGE_SIZE));
        }

        // Parse encrypted payload
        let encrypted_bytes = &bytes[HEADER_SIZE..HEADER_SIZE + payload_len];
        let encrypted = EncryptedMessage::from_bytes(encrypted_bytes)
            .map_err(|e| ProtocolError::InvalidFormat(format!("Invalid encrypted message: {}", e)))?;

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
            return Err(ProtocolError::InvalidFormat("Frame too short for header".to_string()));
        }

        let version = u16::from_le_bytes([bytes[0], bytes[1]]);
        let message_type = bytes[2].try_into()?;
        let message_id = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
        let timestamp = u64::from_le_bytes(bytes[12..20].try_into().unwrap());

        Ok(MessageHeader {
            version,
            message_type,
            message_id,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::MessageType;
    use rand::{rngs::StdRng, RngCore, SeedableRng};

    fn random_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        StdRng::from_os_rng().fill_bytes(&mut key);
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
}
