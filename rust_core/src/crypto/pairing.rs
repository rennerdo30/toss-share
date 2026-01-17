//! Device pairing protocol
//!
//! Implements secure device pairing using ephemeral key exchange.

#![allow(dead_code)]

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{derive_key, DerivedKeyPurpose, EphemeralKeyPair, KEY_SIZE};
use crate::error::CryptoError;

/// Pairing session duration in seconds (5 minutes)
const PAIRING_TIMEOUT_SECS: u64 = 300;

/// Information about a pairing session (for display/sharing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInfo {
    /// 6-digit pairing code
    pub code: String,
    /// QR code data (JSON with code + public key)
    pub qr_data: String,
    /// When the pairing session expires (Unix timestamp)
    pub expires_at: u64,
    /// Public key for key exchange (base64)
    pub public_key: String,
}

/// Pairing session state
pub struct PairingSession {
    code: String,
    ephemeral: EphemeralKeyPair,
    expires_at: u64,
}

/// QR code payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPayload {
    /// Protocol version
    pub v: u16,
    /// 6-digit code
    pub code: String,
    /// Public key (base64)
    pub pk: String,
    /// Device name
    pub name: String,
}

impl PairingSession {
    /// Create a new pairing session
    pub fn new(_device_name: &str) -> Self {
        let code = generate_pairing_code();
        let ephemeral = EphemeralKeyPair::generate();
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + PAIRING_TIMEOUT_SECS;

        Self {
            code,
            ephemeral,
            expires_at,
        }
    }

    /// Get the pairing code
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        self.ephemeral.public_key_bytes()
    }

    /// Get pairing info for display
    pub fn info(&self, device_name: &str) -> PairingInfo {
        let public_key = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            self.ephemeral.public_key_bytes(),
        );

        let qr_payload = QrPayload {
            v: crate::PROTOCOL_VERSION,
            code: self.code.clone(),
            pk: public_key.clone(),
            name: device_name.to_string(),
        };
        let qr_data = serde_json::to_string(&qr_payload).unwrap();

        PairingInfo {
            code: self.code.clone(),
            qr_data,
            expires_at: self.expires_at,
            public_key,
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }

    /// Complete pairing with peer's public key and verification code
    /// Returns the derived session key
    pub fn complete(
        self,
        peer_public_key: &[u8; 32],
        peer_code: &str,
    ) -> Result<[u8; KEY_SIZE], CryptoError> {
        // Check expiration
        if self.is_expired() {
            return Err(CryptoError::SessionExpired);
        }

        // Verify code matches
        if !constant_time_eq(self.code.as_bytes(), peer_code.as_bytes()) {
            return Err(CryptoError::PairingFailed(
                "Invalid pairing code".to_string(),
            ));
        }

        // Derive shared secret
        let shared_secret = self.ephemeral.derive_shared_secret(peer_public_key);

        // Derive session key from shared secret
        let session_key = derive_key(
            shared_secret.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        )?;

        Ok(session_key)
    }

    /// Complete pairing from QR code data
    pub fn complete_from_qr(
        self,
        qr_data: &str,
    ) -> Result<([u8; KEY_SIZE], String, String), CryptoError> {
        let payload: QrPayload = serde_json::from_str(qr_data)
            .map_err(|e| CryptoError::PairingFailed(format!("Invalid QR data: {}", e)))?;

        // Decode public key
        let peer_public_key: [u8; 32] =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &payload.pk)
                .map_err(|e| CryptoError::PairingFailed(format!("Invalid public key: {}", e)))?
                .try_into()
                .map_err(|_| CryptoError::PairingFailed("Invalid public key length".to_string()))?;

        let session_key = self.complete(&peer_public_key, &payload.code)?;

        Ok((session_key, payload.name, payload.pk))
    }

    /// Complete pairing with peer's public key only (code already verified via relay/mDNS)
    /// This is used when finding a device via the pairing coordinator
    pub fn complete_with_peer_key(
        self,
        peer_public_key: &[u8; 32],
    ) -> Result<[u8; KEY_SIZE], CryptoError> {
        // Check expiration
        if self.is_expired() {
            return Err(CryptoError::SessionExpired);
        }

        // Derive shared secret using X25519
        let shared_secret = self.ephemeral.derive_shared_secret(peer_public_key);

        // Derive session key from shared secret
        let session_key = derive_key(
            shared_secret.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        )?;

        Ok(session_key)
    }
}

/// Generate a 6-digit pairing code
fn generate_pairing_code() -> String {
    let mut rng = StdRng::from_entropy();
    let code: u32 = rng.gen_range(0..1_000_000);
    format!("{:06}", code)
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Parse QR code data and extract peer info without completing pairing
pub fn parse_qr_data(qr_data: &str) -> Result<QrPayload, CryptoError> {
    serde_json::from_str(qr_data)
        .map_err(|e| CryptoError::PairingFailed(format!("Invalid QR data: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_code_format() {
        for _ in 0..100 {
            let code = generate_pairing_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn test_pairing_session_info() {
        let session = PairingSession::new("Test Device");
        let info = session.info("Test Device");

        assert_eq!(info.code.len(), 6);
        assert!(!info.qr_data.is_empty());
        assert!(!info.public_key.is_empty());
    }

    #[test]
    fn test_successful_pairing() {
        // Device A creates a session
        let session_a = PairingSession::new("Device A");
        let code_a = session_a.code().to_string();
        let _public_key_a = *session_a.public_key_bytes();

        // Device B creates a session (normally would scan QR)
        let session_b = PairingSession::new("Device B");
        let public_key_b = *session_b.public_key_bytes();

        // Both complete with each other's keys and A's code
        let key_a = session_a.complete(&public_key_b, &code_a).unwrap();

        // For a real pairing, B would use A's code from QR
        // Here we just verify the key derivation works
        assert_eq!(key_a.len(), KEY_SIZE);
    }

    #[test]
    fn test_wrong_code_fails() {
        let session = PairingSession::new("Test");
        let fake_public_key = [0u8; 32];

        let result = session.complete(&fake_public_key, "000000");
        // Should fail because code doesn't match
        assert!(result.is_err());
    }

    #[test]
    fn test_qr_payload_serialization() {
        let session = PairingSession::new("My Device");
        let info = session.info("My Device");

        let payload: QrPayload = serde_json::from_str(&info.qr_data).unwrap();
        assert_eq!(payload.code, info.code);
        assert_eq!(payload.name, "My Device");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }
}
