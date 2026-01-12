//! Device identity using Ed25519 signatures

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::error::CryptoError;

/// Device identity containing Ed25519 signing keys
pub struct DeviceIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    device_id: [u8; 32],
}

impl DeviceIdentity {
    /// Generate a new random device identity
    pub fn generate() -> Result<Self, CryptoError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Device ID is SHA-256 hash of public key
        let mut hasher = Sha256::new();
        hasher.update(verifying_key.as_bytes());
        let device_id: [u8; 32] = hasher.finalize().into();

        Ok(Self {
            signing_key,
            verifying_key,
            device_id,
        })
    }

    /// Load identity from bytes (private key)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey);
        }

        let signing_key = SigningKey::try_from(bytes)
            .map_err(|_| CryptoError::InvalidKey)?;
        let verifying_key = signing_key.verifying_key();

        let mut hasher = Sha256::new();
        hasher.update(verifying_key.as_bytes());
        let device_id: [u8; 32] = hasher.finalize().into();

        Ok(Self {
            signing_key,
            verifying_key,
            device_id,
        })
    }

    /// Export private key bytes for secure storage
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the device ID (hash of public key)
    pub fn device_id(&self) -> &[u8; 32] {
        &self.device_id
    }

    /// Get device ID as hex string
    pub fn device_id_hex(&self) -> String {
        hex::encode(self.device_id)
    }

    /// Get the public key bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verify a signature made by this identity
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig).is_ok()
    }

    /// Verify a signature from another device
    pub fn verify_from_public_key(
        public_key: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(public_key) {
            Ok(k) => k,
            Err(_) => return false,
        };
        let sig = Signature::from_bytes(signature);
        verifying_key.verify(message, &sig).is_ok()
    }
}

impl Clone for DeviceIdentity {
    fn clone(&self) -> Self {
        Self::from_bytes(&self.to_bytes()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let identity = DeviceIdentity::generate().unwrap();
        assert_eq!(identity.device_id().len(), 32);
        assert_eq!(identity.public_key().len(), 32);
    }

    #[test]
    fn test_roundtrip() {
        let identity = DeviceIdentity::generate().unwrap();
        let bytes = identity.to_bytes();
        let restored = DeviceIdentity::from_bytes(&bytes).unwrap();

        assert_eq!(identity.device_id(), restored.device_id());
        assert_eq!(identity.public_key(), restored.public_key());
    }

    #[test]
    fn test_sign_verify() {
        let identity = DeviceIdentity::generate().unwrap();
        let message = b"Hello, World!";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));

        // Wrong message should fail
        assert!(!identity.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_verify_from_public_key() {
        let identity = DeviceIdentity::generate().unwrap();
        let message = b"Test message";
        let signature = identity.sign(message);

        let public_key = identity.public_key();
        assert!(DeviceIdentity::verify_from_public_key(&public_key, message, &signature));
    }
}
