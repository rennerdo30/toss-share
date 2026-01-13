//! Cryptographic operations for Toss
//!
//! This module provides:
//! - Device identity (Ed25519 signing keys)
//! - Key exchange (X25519)
//! - Symmetric encryption (AES-256-GCM)
//! - Key derivation (HKDF-SHA256)
//! - Device pairing protocol

mod identity;
mod kdf;
mod key_exchange;
mod pairing;
mod symmetric;

pub use identity::DeviceIdentity;
pub use kdf::{derive_key, DerivedKeyPurpose};
pub use key_exchange::{EphemeralKeyPair, SharedSecret};
pub use pairing::{PairingInfo, PairingSession};
pub use symmetric::{decrypt, encrypt, EncryptedMessage};

/// Size of AES-256 key in bytes
pub const KEY_SIZE: usize = 32;

/// Size of GCM nonce in bytes
pub const NONCE_SIZE: usize = 12;

/// Size of GCM authentication tag in bytes
pub const TAG_SIZE: usize = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_crypto_roundtrip() {
        // Generate identities for two devices
        let _alice_identity = DeviceIdentity::generate().unwrap();
        let _bob_identity = DeviceIdentity::generate().unwrap();

        // Generate ephemeral keys for key exchange
        let alice_ephemeral = EphemeralKeyPair::generate();
        let bob_ephemeral = EphemeralKeyPair::generate();

        // Save public keys before consuming the key pairs
        let alice_public = *alice_ephemeral.public_key_bytes();
        let bob_public = *bob_ephemeral.public_key_bytes();

        // Exchange public keys and derive shared secret
        let alice_shared = alice_ephemeral.derive_shared_secret(&bob_public);
        let bob_shared = bob_ephemeral.derive_shared_secret(&alice_public);

        // Shared secrets should match
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());

        // Derive session key
        let alice_session_key = derive_key(
            alice_shared.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        )
        .unwrap();

        let bob_session_key = derive_key(
            bob_shared.as_bytes(),
            DerivedKeyPurpose::SessionEncryption,
            None,
        )
        .unwrap();

        assert_eq!(alice_session_key, bob_session_key);

        // Encrypt a message
        let plaintext = b"Hello from Alice!";
        let aad = b"additional data";
        let encrypted = encrypt(&alice_session_key, plaintext, aad).unwrap();

        // Decrypt with Bob's key
        let decrypted = decrypt(&bob_session_key, &encrypted, aad).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_signature_verification() {
        let identity = DeviceIdentity::generate().unwrap();
        let message = b"Sign this message";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));

        // Tampered message should fail
        let tampered = b"Sign this messag3";
        assert!(!identity.verify(tampered, &signature));
    }
}
