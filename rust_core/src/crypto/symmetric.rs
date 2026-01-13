//! AES-256-GCM symmetric encryption

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use crate::error::CryptoError;
use super::{KEY_SIZE, NONCE_SIZE, TAG_SIZE};

/// Encrypted message with nonce and authentication tag
#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    /// 12-byte nonce (IV)
    pub nonce: [u8; NONCE_SIZE],
    /// Ciphertext with appended authentication tag
    pub ciphertext: Vec<u8>,
}

impl EncryptedMessage {
    /// Serialize to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(NONCE_SIZE + self.ciphertext.len());
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() < NONCE_SIZE + TAG_SIZE {
            return Err(CryptoError::Decryption("Message too short".to_string()));
        }

        let nonce: [u8; NONCE_SIZE] = bytes[..NONCE_SIZE]
            .try_into()
            .map_err(|_| CryptoError::Decryption("Invalid nonce".to_string()))?;
        let ciphertext = bytes[NONCE_SIZE..].to_vec();

        Ok(Self { nonce, ciphertext })
    }
}

/// Encrypt plaintext using AES-256-GCM
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `plaintext` - Data to encrypt
/// * `aad` - Additional authenticated data (not encrypted, but authenticated)
pub fn encrypt(key: &[u8; KEY_SIZE], plaintext: &[u8], aad: &[u8]) -> Result<EncryptedMessage, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    StdRng::from_os_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt with AAD
    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload { msg: plaintext, aad })
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    Ok(EncryptedMessage {
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Decrypt ciphertext using AES-256-GCM
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `message` - Encrypted message with nonce
/// * `aad` - Additional authenticated data (must match encryption)
pub fn decrypt(key: &[u8; KEY_SIZE], message: &EncryptedMessage, aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    let nonce = Nonce::from_slice(&message.nonce);

    cipher
        .decrypt(nonce, aes_gcm::aead::Payload { msg: &message.ciphertext, aad })
        .map_err(|e| CryptoError::Decryption(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; KEY_SIZE] {
        let mut key = [0u8; KEY_SIZE];
        rand::rngs::StdRng::from_os_rng().fill_bytes(&mut key);
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"Hello, World!";
        let aad = b"context";

        let encrypted = encrypt(&key, plaintext, aad).unwrap();
        let decrypted = decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key1 = test_key();
        let key2 = test_key();
        let plaintext = b"Secret message";
        let aad = b"";

        let encrypted = encrypt(&key1, plaintext, aad).unwrap();
        let result = decrypt(&key2, &encrypted, aad);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_wrong_aad_fails() {
        let key = test_key();
        let plaintext = b"Secret message";

        let encrypted = encrypt(&key, plaintext, b"correct aad").unwrap();
        let result = decrypt(&key, &encrypted, b"wrong aad");

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let plaintext = b"Secret message";
        let aad = b"";

        let mut encrypted = encrypt(&key, plaintext, aad).unwrap();
        encrypted.ciphertext[0] ^= 0xFF; // Tamper with first byte

        let result = decrypt(&key, &encrypted, aad);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let key = test_key();
        let plaintext = b"Test serialization";
        let aad = b"";

        let encrypted = encrypt(&key, plaintext, aad).unwrap();
        let bytes = encrypted.to_bytes();
        let restored = EncryptedMessage::from_bytes(&bytes).unwrap();
        let decrypted = decrypt(&key, &restored, aad).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_unique_nonces() {
        let key = test_key();
        let plaintext = b"Same message";
        let aad = b"";

        let encrypted1 = encrypt(&key, plaintext, aad).unwrap();
        let encrypted2 = encrypt(&key, plaintext, aad).unwrap();

        // Same plaintext should produce different ciphertexts due to random nonce
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
    }
}
