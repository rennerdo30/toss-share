//! Integration tests for the Toss Relay Server
//!
//! These tests verify the API endpoints and WebSocket functionality.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::Rng;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Helper to generate test signing key pair
fn generate_keypair() -> (SigningKey, String, String) {
    // Generate a random 32-byte seed using rand 0.10's API and build the
    // signing key from it. This avoids depending on ed25519-dalek's pinned
    // rand_core 0.6 OsRng, which is incompatible with rand 0.10's RNG traits.
    let mut secret_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut secret_bytes);
    let signing_key = SigningKey::from_bytes(&secret_bytes);
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

#[cfg(test)]
mod admin_auth_integration_tests {
    use toss_relay::{Config, TestServer};

    /// Create a test config with admin credentials
    fn admin_config() -> Config {
        let password = "test_password_123";
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();

        Config {
            admin_username: Some("admin".to_string()),
            admin_password_hash: Some(password_hash),
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn test_admin_login_page_without_credentials() {
        // Start test server without admin credentials
        let server = TestServer::start()
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::new();

        // Login page should return 404 when admin is not configured
        let response = client
            .get(server.url("/admin/login"))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Login page should return 404 when admin not configured"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_login_page_with_credentials() {
        // Start test server with admin credentials
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::new();

        // Login page should be accessible
        let response = client
            .get(server.url("/admin/login"))
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status().is_success(),
            "Login page should be accessible: {}",
            response.status()
        );

        let body = response.text().await.unwrap();
        assert!(
            body.contains("Toss Relay Admin"),
            "Login page should contain title"
        );
        assert!(
            body.contains("username"),
            "Login page should have username field"
        );
        assert!(
            body.contains("password"),
            "Login page should have password field"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_login_invalid_credentials() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::new();

        // Try to login with wrong credentials
        let response = client
            .post(server.url("/admin/login"))
            .form(&[("username", "wrong"), ("password", "wrong")])
            .send()
            .await
            .expect("Failed to send request");

        let body = response.text().await.unwrap();
        assert!(
            body.contains("Invalid username or password"),
            "Should show error message for invalid credentials"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_login_valid_credentials() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        // Create a client that follows redirects
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        // Login with valid credentials
        let response = client
            .post(server.url("/admin/login"))
            .form(&[("username", "admin"), ("password", "test_password_123")])
            .send()
            .await
            .expect("Failed to send request");

        // Should redirect to dashboard
        assert_eq!(
            response.status(),
            reqwest::StatusCode::SEE_OTHER,
            "Should redirect after successful login"
        );

        // Should have set a session cookie
        let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
        assert!(!cookies.is_empty(), "Should set session cookie");

        let cookie_str = cookies[0].to_str().unwrap();
        assert!(
            cookie_str.contains("admin_session="),
            "Cookie should contain session token"
        );
        assert!(cookie_str.contains("HttpOnly"), "Cookie should be HttpOnly");
        assert!(
            cookie_str.contains("SameSite=Strict"),
            "Cookie should have SameSite=Strict"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_dashboard_requires_auth() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        // Try to access dashboard without authentication
        let response = client
            .get(server.url("/admin"))
            .send()
            .await
            .expect("Failed to send request");

        // Should redirect to login
        assert_eq!(
            response.status(),
            reqwest::StatusCode::SEE_OTHER,
            "Should redirect to login when not authenticated"
        );

        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(location, "/admin/login", "Should redirect to login page");

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_dashboard_with_valid_session() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        // Create client with cookie jar
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap();

        // Login first
        let _login_response = client
            .post(server.url("/admin/login"))
            .form(&[("username", "admin"), ("password", "test_password_123")])
            .send()
            .await
            .expect("Failed to login");

        // Now access dashboard with the session cookie
        let response = client
            .get(server.url("/admin"))
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status().is_success(),
            "Should be able to access dashboard with valid session"
        );

        let body = response.text().await.unwrap();
        assert!(
            body.contains("Dashboard") || body.contains("Online Devices"),
            "Dashboard should show stats"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_logout() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        // Logout should redirect to login and clear the cookie
        let response = client
            .post(server.url("/admin/logout"))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::SEE_OTHER,
            "Should redirect after logout"
        );

        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(location, "/admin/login", "Should redirect to login page");

        // Cookie should be cleared (Max-Age=0)
        let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
        if !cookies.is_empty() {
            let cookie_str = cookies[0].to_str().unwrap();
            assert!(
                cookie_str.contains("Max-Age=0"),
                "Session cookie should be cleared with Max-Age=0"
            );
        }

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_stats_api_requires_auth() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::new();

        // Try to access stats API without authentication
        let response = client
            .get(server.url("/api/admin/stats"))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "Stats API should require authentication"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_stats_api_with_auth() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        // Create client with cookie jar
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap();

        // Login first
        let _login_response = client
            .post(server.url("/admin/login"))
            .form(&[("username", "admin"), ("password", "test_password_123")])
            .send()
            .await
            .expect("Failed to login");

        // Access stats API with session
        let response = client
            .get(server.url("/api/admin/stats"))
            .send()
            .await
            .expect("Failed to send request");

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        assert!(
            status.is_success(),
            "Stats API should work with valid session. Status: {}, Body: {}",
            status,
            body_text
        );
        let body: serde_json::Value = serde_json::from_str(&body_text).expect("Valid JSON");
        assert!(
            body.get("online_devices").is_some(),
            "Stats should include online_devices"
        );
        assert!(
            body.get("total_devices").is_some(),
            "Stats should include total_devices"
        );
        assert!(
            body.get("uptime_seconds").is_some(),
            "Stats should include uptime_seconds"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_admin_devices_page_requires_auth() {
        let config = admin_config();
        let server = TestServer::start_with_config(config)
            .await
            .expect("Failed to start test server");

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        // Try to access devices page without authentication
        let response = client
            .get(server.url("/admin/devices"))
            .send()
            .await
            .expect("Failed to send request");

        // Should redirect to login
        assert_eq!(
            response.status(),
            reqwest::StatusCode::SEE_OTHER,
            "Devices page should redirect to login when not authenticated"
        );

        server.shutdown().await;
    }
}
