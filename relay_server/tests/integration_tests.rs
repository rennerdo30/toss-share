//! Integration tests for the Toss Relay Server
//!
//! These tests verify the API endpoints and WebSocket functionality.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper to generate test signing key pair
fn generate_keypair() -> (SigningKey, String, String) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let public_key_base64 =
        base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    let device_id = hex::encode(&verifying_key.to_bytes()[..16]);

    (signing_key, device_id, public_key_base64)
}

/// Helper to create a signed registration request
fn create_register_request(
    signing_key: &SigningKey,
    device_id: &str,
    public_key: &str,
    device_name: &str,
) -> Value {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = format!("register:{}:{}", device_id, timestamp);
    let signature = signing_key.sign(message.as_bytes());
    let signature_base64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    json!({
        "device_id": device_id,
        "public_key": public_key,
        "device_name": device_name,
        "timestamp": timestamp,
        "signature": signature_base64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let (_signing_key, device_id, public_key) = generate_keypair();

        // Device ID should be 32 hex characters (16 bytes)
        assert_eq!(device_id.len(), 32);

        // Public key should be base64 encoded 32 bytes
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&public_key)
            .unwrap();
        assert_eq!(decoded.len(), 32);
    }

    #[test]
    fn test_create_register_request() {
        let (signing_key, device_id, public_key) = generate_keypair();

        let request = create_register_request(&signing_key, &device_id, &public_key, "Test Device");

        assert_eq!(request["device_id"], device_id);
        assert_eq!(request["device_name"], "Test Device");
        assert!(request["timestamp"].is_number());
        assert!(request["signature"].is_string());
    }

    #[test]
    fn test_signature_verification() {
        let (signing_key, device_id, _) = generate_keypair();
        let verifying_key = signing_key.verifying_key();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let message = format!("register:{}:{}", device_id, timestamp);
        let signature = signing_key.sign(message.as_bytes());

        // Verification should succeed
        assert!(verifying_key
            .verify_strict(message.as_bytes(), &signature)
            .is_ok());

        // Wrong message should fail
        let wrong_message = format!("register:{}:{}", device_id, timestamp + 1);
        assert!(verifying_key
            .verify_strict(wrong_message.as_bytes(), &signature)
            .is_err());
    }
}

#[cfg(test)]
mod api_integration_tests {
    use super::*;
    use toss_relay::TestServer;

    #[tokio::test]
    async fn test_device_registration_flow() {
        // Start test server
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");

        // Generate keypair
        let (signing_key, device_id, public_key) = generate_keypair();

        // Create registration request
        let request = create_register_request(&signing_key, &device_id, &public_key, "Test Device");

        // Send registration request
        let client = reqwest::Client::new();
        let response = client
            .post(server.url("/api/register"))
            .json(&request)
            .send()
            .await
            .expect("Failed to send request");

        // Check response
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(json!({}));

        // Should succeed with a token
        assert!(
            status.is_success(),
            "Registration failed with status {}: {:?}",
            status,
            body
        );
        assert!(
            body.get("token").is_some() || body.get("success").is_some(),
            "Response should contain token or success: {:?}",
            body
        );

        // Cleanup
        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        // Start test server
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");

        // Check health endpoint
        let client = reqwest::Client::new();
        let response = client
            .get(server.url("/api/health"))
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status().is_success(),
            "Health check should succeed"
        );

        // Cleanup
        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_invalid_registration_signature() {
        // Start test server
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");

        // Generate keypair
        let (_signing_key, device_id, public_key) = generate_keypair();

        // Create request with invalid signature
        let request = json!({
            "device_id": device_id,
            "public_key": public_key,
            "device_name": "Test Device",
            "timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "signature": "invalid_signature_base64"
        });

        // Send registration request
        let client = reqwest::Client::new();
        let response = client
            .post(server.url("/api/register"))
            .json(&request)
            .send()
            .await
            .expect("Failed to send request");

        // Should fail with bad request or unauthorized
        assert!(
            response.status().is_client_error(),
            "Invalid signature should be rejected"
        );

        // Cleanup
        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        // Start test server
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");

        // Generate keypair
        let (signing_key, device_id, public_key) = generate_keypair();

        // Create registration request
        let request = create_register_request(&signing_key, &device_id, &public_key, "Test Device");

        let client = reqwest::Client::new();

        // First registration should succeed
        let response1 = client
            .post(server.url("/api/register"))
            .json(&request)
            .send()
            .await
            .expect("Failed to send first request");
        assert!(
            response1.status().is_success(),
            "First registration should succeed"
        );

        // Create new request with fresh timestamp for second registration
        let request2 =
            create_register_request(&signing_key, &device_id, &public_key, "Test Device Updated");

        // Second registration should also succeed (update device name)
        let response2 = client
            .post(server.url("/api/register"))
            .json(&request2)
            .send()
            .await
            .expect("Failed to send second request");

        // Re-registration with same device should succeed (upsert behavior)
        assert!(
            response2.status().is_success(),
            "Re-registration should succeed with upsert behavior"
        );

        // Cleanup
        server.shutdown().await;
    }
}
