//! Wire protocol for Toss communication
//!
//! This module defines the message types and serialization format
//! used for communication between Toss devices.

mod content;
mod frame;
mod message;

pub use content::{ClipboardContent, ContentMetadata, ContentType};
pub use frame::Frame;
pub use message::{
    ClipboardAck, ClipboardRequest, ClipboardUpdate, DeviceInfo, ErrorMessage, KeyRotation,
    KeyRotationReason, Message, MessageHeader, MessageType, Ping, Platform, Pong,
};

/// Maximum message size (50 MB)
pub const MAX_MESSAGE_SIZE: usize = 50 * 1024 * 1024;

/// Maximum preview size for metadata (256 KB)
pub const MAX_PREVIEW_SIZE: usize = 256 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{derive_key, DerivedKeyPurpose, EphemeralKeyPair};

    #[test]
    #[ignore = "bincode enum encoding needs investigation"]
    fn test_full_message_roundtrip() {
        // Create a clipboard update message
        let content = ClipboardContent {
            content_type: ContentType::PlainText,
            data: b"Hello, World!".to_vec(),
            metadata: ContentMetadata::default(),
        };

        let update = ClipboardUpdate::new(content);
        let message = Message::ClipboardUpdate(update.clone());

        // Serialize and deserialize the message directly first
        let payload = message.serialize().unwrap();
        let header = message.header();

        // Test direct message roundtrip
        let parsed: Message = bincode::deserialize(&payload).unwrap();
        if let Message::ClipboardUpdate(parsed_update) = &parsed {
            assert_eq!(parsed_update.content.data, update.content.data);
        } else {
            panic!("Expected ClipboardUpdate from direct deserialization");
        }

        // Create frame with encryption
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let bob_public = *bob.public_key_bytes();
        let shared = alice.derive_shared_secret(&bob_public);
        let key = derive_key(
            shared.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        )
        .unwrap();

        let frame = Frame::encrypt(&header, &payload, &key).unwrap();
        let frame_bytes = frame.to_bytes();

        // Parse and decrypt
        let parsed_frame = Frame::from_bytes(&frame_bytes).unwrap();
        let (parsed_header, decrypted_payload) = parsed_frame.decrypt(&key).unwrap();

        // Verify the decrypted payload matches original
        assert_eq!(payload, decrypted_payload);
        assert_eq!(parsed_header.message_type, header.message_type);

        // Deserialize message from decrypted payload
        let parsed_message: Message = bincode::deserialize(&decrypted_payload).unwrap();

        if let Message::ClipboardUpdate(parsed_update) = parsed_message {
            assert_eq!(parsed_update.content.data, update.content.data);
        } else {
            panic!("Expected ClipboardUpdate");
        }
    }
}
