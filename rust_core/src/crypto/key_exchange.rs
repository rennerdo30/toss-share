//! X25519 key exchange

use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret as X25519SharedSecret};

/// Ephemeral key pair for X25519 key exchange
pub struct EphemeralKeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl EphemeralKeyPair {
    /// Generate a new ephemeral key pair
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        self.public.as_bytes()
    }

    /// Derive shared secret from peer's public key
    /// Consumes self because EphemeralSecret can only be used once
    pub fn derive_shared_secret(self, peer_public_key: &[u8; 32]) -> SharedSecret {
        let peer_public = PublicKey::from(*peer_public_key);
        let shared = self.secret.diffie_hellman(&peer_public);
        SharedSecret { inner: shared }
    }
}

/// Shared secret from X25519 key exchange
pub struct SharedSecret {
    inner: X25519SharedSecret,
}

impl SharedSecret {
    /// Get the raw shared secret bytes
    /// Note: You should use HKDF to derive actual encryption keys from this
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.inner.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange() {
        // Alice generates her ephemeral key pair
        let alice = EphemeralKeyPair::generate();
        let alice_public = *alice.public_key_bytes();

        // Bob generates his ephemeral key pair
        let bob = EphemeralKeyPair::generate();
        let bob_public = *bob.public_key_bytes();

        // They exchange public keys and derive shared secrets
        let alice_shared = alice.derive_shared_secret(&bob_public);
        let bob_shared = bob.derive_shared_secret(&alice_public);

        // Shared secrets should be identical
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_different_keys_different_secrets() {
        let alice = EphemeralKeyPair::generate();
        let bob = EphemeralKeyPair::generate();
        let charlie = EphemeralKeyPair::generate();

        let alice_bob = alice.derive_shared_secret(bob.public_key_bytes());
        let charlie_bob = charlie.derive_shared_secret(bob.public_key_bytes());

        // Different pairs should have different secrets
        assert_ne!(alice_bob.as_bytes(), charlie_bob.as_bytes());
    }
}
