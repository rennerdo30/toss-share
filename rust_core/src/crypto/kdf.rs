//! Key derivation using HKDF-SHA256
//!
//! Provides HKDF-based key derivation for session keys and other purposes.

#![allow(dead_code)]

use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::CryptoError;
use super::KEY_SIZE;

/// Purpose of derived key (used as context in HKDF)
#[derive(Debug, Clone, Copy)]
pub enum DerivedKeyPurpose {
    /// Key for encrypting session messages
    SessionEncryption,
    /// Key for authenticating messages
    MessageAuthentication,
    /// Key for encrypting stored data
    StorageEncryption,
}

impl DerivedKeyPurpose {
    fn info_bytes(&self) -> &[u8] {
        match self {
            DerivedKeyPurpose::SessionEncryption => b"toss-session-encryption-v1",
            DerivedKeyPurpose::MessageAuthentication => b"toss-message-auth-v1",
            DerivedKeyPurpose::StorageEncryption => b"toss-storage-encryption-v1",
        }
    }
}

/// Derive a key from input key material using HKDF-SHA256
///
/// # Arguments
/// * `ikm` - Input key material (e.g., shared secret from X25519)
/// * `purpose` - What the derived key will be used for
/// * `salt` - Optional salt (if None, uses empty salt)
pub fn derive_key(
    ikm: &[u8],
    purpose: DerivedKeyPurpose,
    salt: Option<&[u8]>,
) -> Result<[u8; KEY_SIZE], CryptoError> {
    let salt = salt.unwrap_or(&[]);
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);

    let mut okm = [0u8; KEY_SIZE];
    hk.expand(purpose.info_bytes(), &mut okm)
        .map_err(|e| CryptoError::KeyDerivation(e.to_string()))?;

    Ok(okm)
}

/// Derive multiple keys from the same input key material
pub fn derive_keys(
    ikm: &[u8],
    purposes: &[DerivedKeyPurpose],
    salt: Option<&[u8]>,
) -> Result<Vec<[u8; KEY_SIZE]>, CryptoError> {
    purposes
        .iter()
        .map(|purpose| derive_key(ikm, *purpose, salt))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key() {
        let ikm = b"input key material";
        let key = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, None).unwrap();

        assert_eq!(key.len(), KEY_SIZE);
    }

    #[test]
    fn test_deterministic_derivation() {
        let ikm = b"same input";
        let key1 = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, None).unwrap();
        let key2 = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, None).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_different_purposes_different_keys() {
        let ikm = b"input key material";
        let key1 = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, None).unwrap();
        let key2 = derive_key(ikm, DerivedKeyPurpose::MessageAuthentication, None).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_different_salts_different_keys() {
        let ikm = b"input key material";
        let key1 = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, Some(b"salt1")).unwrap();
        let key2 = derive_key(ikm, DerivedKeyPurpose::SessionEncryption, Some(b"salt2")).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_multiple_keys() {
        let ikm = b"input key material";
        let purposes = [
            DerivedKeyPurpose::SessionEncryption,
            DerivedKeyPurpose::MessageAuthentication,
        ];

        let keys = derive_keys(ikm, &purposes, None).unwrap();

        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0], keys[1]);
    }
}
