//! Admin authentication for the dashboard

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::AppState;

const SESSION_COOKIE_NAME: &str = "admin_session";
const SESSION_EXPIRATION_SECS: u64 = 86400; // 24 hours

// Rate limiting constants
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const LOCKOUT_DURATION_SECS: u64 = 300; // 5 minutes
const ATTEMPT_WINDOW_SECS: u64 = 300; // 5 minutes

/// Rate limiter for login attempts
pub struct LoginRateLimiter {
    attempts: RwLock<HashMap<IpAddr, Vec<Instant>>>,
    lockouts: RwLock<HashMap<IpAddr, Instant>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: RwLock::new(HashMap::new()),
            lockouts: RwLock::new(HashMap::new()),
        }
    }

    /// Check if an IP is currently locked out
    pub fn is_locked_out(&self, ip: &IpAddr) -> bool {
        let lockouts = self.lockouts.read().unwrap();
        if let Some(lockout_time) = lockouts.get(ip) {
            if lockout_time.elapsed().as_secs() < LOCKOUT_DURATION_SECS {
                return true;
            }
        }
        false
    }

    /// Record a failed login attempt and return whether the IP should be locked out
    pub fn record_failed_attempt(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(ATTEMPT_WINDOW_SECS);

        let mut attempts = self.attempts.write().unwrap();
        let ip_attempts = attempts.entry(ip).or_default();

        // Remove old attempts outside the window
        ip_attempts.retain(|t| now.duration_since(*t) < window);

        // Add the new attempt
        ip_attempts.push(now);

        // Check if we've exceeded the limit
        if ip_attempts.len() as u32 >= MAX_LOGIN_ATTEMPTS {
            drop(attempts);
            let mut lockouts = self.lockouts.write().unwrap();
            lockouts.insert(ip, now);
            tracing::warn!("IP {} locked out due to too many failed login attempts", ip);
            return true;
        }

        false
    }

    /// Clear attempts and lockout for an IP on successful login
    pub fn clear_attempts(&self, ip: &IpAddr) {
        let mut attempts = self.attempts.write().unwrap();
        attempts.remove(ip);
        drop(attempts);

        let mut lockouts = self.lockouts.write().unwrap();
        lockouts.remove(ip);
    }

    /// Get remaining lockout time in seconds for an IP
    pub fn remaining_lockout_secs(&self, ip: &IpAddr) -> Option<u64> {
        let lockouts = self.lockouts.read().unwrap();
        if let Some(lockout_time) = lockouts.get(ip) {
            let elapsed = lockout_time.elapsed().as_secs();
            if elapsed < LOCKOUT_DURATION_SECS {
                return Some(LOCKOUT_DURATION_SECS - elapsed);
            }
        }
        None
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global rate limiter instance
static RATE_LIMITER: std::sync::LazyLock<LoginRateLimiter> =
    std::sync::LazyLock::new(LoginRateLimiter::new);

/// Login page template
#[derive(Template)]
#[template(path = "admin/login.html")]
struct LoginTemplate {
    error: String,
    has_error: bool,
}

/// Login form data
#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// Extract client IP from request headers (handles proxies)
fn extract_client_ip(headers: &axum::http::HeaderMap) -> IpAddr {
    // Try X-Forwarded-For first (for reverse proxy setups)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // X-Forwarded-For can contain multiple IPs, take the first (client IP)
            if let Some(first_ip) = xff_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse() {
                    return ip;
                }
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse() {
                return ip;
            }
        }
    }

    // Default to localhost if we can't determine IP
    "127.0.0.1".parse().unwrap()
}

/// Display login page
pub async fn login_page(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // If admin is not enabled, return 404
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    // Check if IP is locked out
    let client_ip = extract_client_ip(&headers);
    if let Some(remaining) = RATE_LIMITER.remaining_lockout_secs(&client_ip) {
        let template = LoginTemplate {
            error: format!(
                "Too many failed attempts. Please try again in {} seconds.",
                remaining
            ),
            has_error: true,
        };
        return Html(template.render().unwrap_or_default()).into_response();
    }

    let template = LoginTemplate {
        error: String::new(),
        has_error: false,
    };
    Html(template.render().unwrap_or_default()).into_response()
}

/// Handle login form submission
pub async fn login_submit(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    let client_ip = extract_client_ip(&headers);

    // Check if IP is locked out
    if RATE_LIMITER.is_locked_out(&client_ip) {
        if let Some(remaining) = RATE_LIMITER.remaining_lockout_secs(&client_ip) {
            let template = LoginTemplate {
                error: format!(
                    "Too many failed attempts. Please try again in {} seconds.",
                    remaining
                ),
                has_error: true,
            };
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Html(template.render().unwrap_or_default()),
            )
                .into_response();
        }
    }

    let expected_username = state.config.admin_username.as_deref().unwrap_or("");
    let password_hash = state.config.admin_password_hash.as_deref().unwrap_or("");

    // Verify credentials using bcrypt
    // bcrypt::verify performs constant-time comparison internally
    let username_matches = constant_time_eq(form.username.as_bytes(), expected_username.as_bytes());
    let password_matches = bcrypt::verify(&form.password, password_hash).unwrap_or(false);

    if username_matches && password_matches {
        // Clear rate limit attempts on successful login
        RATE_LIMITER.clear_attempts(&client_ip);

        // Create session token using HMAC-SHA256
        let session_token = create_session_token(&state.config.session_secret);

        // Set secure cookie and redirect to dashboard
        // Path=/ allows the cookie to be sent for both /admin and /api/admin routes
        let cookie = format!(
            "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            SESSION_COOKIE_NAME, session_token, SESSION_EXPIRATION_SECS
        );

        tracing::info!("Admin login successful from IP: {}", client_ip);

        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(header::SET_COOKIE, cookie)
            .header(header::LOCATION, "/admin")
            .body(axum::body::Body::empty())
            .unwrap()
            .into_response()
    } else {
        // Record failed attempt
        let locked_out = RATE_LIMITER.record_failed_attempt(client_ip);

        tracing::warn!("Failed admin login attempt from IP: {}", client_ip);

        let error_message = if locked_out {
            format!(
                "Too many failed attempts. Please try again in {} seconds.",
                LOCKOUT_DURATION_SECS
            )
        } else {
            "Invalid username or password".to_string()
        };

        let status = if locked_out {
            StatusCode::TOO_MANY_REQUESTS
        } else {
            StatusCode::OK
        };

        let template = LoginTemplate {
            error: error_message,
            has_error: true,
        };
        (status, Html(template.render().unwrap_or_default())).into_response()
    }
}

/// Handle logout
pub async fn logout() -> impl IntoResponse {
    // Clear the session cookie
    let cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        SESSION_COOKIE_NAME
    );

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::SET_COOKIE, cookie)
        .header(header::LOCATION, "/admin/login")
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Check if the request has a valid admin session
pub fn is_authenticated(cookies: &str, session_secret: &str) -> bool {
    // Parse cookies to find session cookie
    for cookie in cookies.split(';') {
        let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
        if parts.len() == 2 && parts[0] == SESSION_COOKIE_NAME {
            return verify_session_token(parts[1], session_secret);
        }
    }
    false
}

/// Create a session token using HMAC-SHA256
fn create_session_token(secret: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Token data: "admin:{timestamp}"
    let data = format!("admin:{}", timestamp);
    let signature = hmac_sha256(secret, &data);

    // Token format: "{timestamp}:{signature}"
    format!("{}:{}", timestamp, signature)
}

/// Verify a session token
fn verify_session_token(token: &str, secret: &str) -> bool {
    let parts: Vec<&str> = token.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }

    let timestamp: u64 = match parts[0].parse() {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Check if token has expired
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now.saturating_sub(timestamp) > SESSION_EXPIRATION_SECS {
        return false;
    }

    // Verify signature using constant-time comparison
    let data = format!("admin:{}", timestamp);
    let expected_signature = hmac_sha256(secret, &data);

    constant_time_eq(parts[1].as_bytes(), expected_signature.as_bytes())
}

/// Compute HMAC-SHA256 signature
fn hmac_sha256(secret: &str, data: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    let result = mac.finalize();

    // Return hex-encoded signature
    hex_encode(result.into_bytes().as_slice())
}

/// Hex encode bytes
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time byte comparison to prevent timing attacks
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

/// Middleware-like function to check auth and redirect if needed
pub fn require_auth(cookies: Option<&str>, session_secret: &str) -> Option<Redirect> {
    let cookies = cookies.unwrap_or("");
    if !is_authenticated(cookies, session_secret) {
        Some(Redirect::to("/admin/login"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_token_creation_and_verification() {
        let secret = "test_secret_key_for_testing";
        let token = create_session_token(secret);

        // Token should be valid
        assert!(verify_session_token(&token, secret));

        // Token should be invalid with wrong secret
        assert!(!verify_session_token(&token, "wrong_secret"));

        // Malformed tokens should be invalid
        assert!(!verify_session_token("invalid", secret));
        assert!(!verify_session_token("", secret));
        assert!(!verify_session_token("abc:def", secret));
    }

    #[test]
    fn test_expired_session_token() {
        let secret = "test_secret";

        // Create a token with a very old timestamp
        let old_timestamp = 0u64; // Unix epoch - definitely expired
        let data = format!("admin:{}", old_timestamp);
        let signature = hmac_sha256(secret, &data);
        let old_token = format!("{}:{}", old_timestamp, signature);

        // Should be invalid due to expiration
        assert!(!verify_session_token(&old_token, secret));
    }

    #[test]
    fn test_hmac_sha256_consistency() {
        let secret = "my_secret";
        let data = "test_data";

        // Same inputs should produce same output
        let sig1 = hmac_sha256(secret, data);
        let sig2 = hmac_sha256(secret, data);
        assert_eq!(sig1, sig2);

        // Different inputs should produce different output
        let sig3 = hmac_sha256(secret, "different_data");
        assert_ne!(sig1, sig3);

        let sig4 = hmac_sha256("different_secret", data);
        assert_ne!(sig1, sig4);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn test_is_authenticated() {
        let secret = "test_secret";
        let token = create_session_token(secret);

        // Valid session cookie
        let cookies = format!("admin_session={}", token);
        assert!(is_authenticated(&cookies, secret));

        // Multiple cookies
        let cookies = format!("other=value; admin_session={}; another=test", token);
        assert!(is_authenticated(&cookies, secret));

        // No session cookie
        assert!(!is_authenticated("other=value", secret));

        // Invalid token
        assert!(!is_authenticated("admin_session=invalid", secret));
    }

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = LoginRateLimiter::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Initially not locked out
        assert!(!limiter.is_locked_out(&ip));

        // Record attempts up to threshold
        for i in 0..MAX_LOGIN_ATTEMPTS - 1 {
            let locked = limiter.record_failed_attempt(ip);
            assert!(!locked, "Should not be locked out after {} attempts", i + 1);
        }

        // The final attempt should trigger lockout
        let locked = limiter.record_failed_attempt(ip);
        assert!(locked, "Should be locked out after max attempts");

        // Now should be locked out
        assert!(limiter.is_locked_out(&ip));

        // Should have remaining lockout time
        let remaining = limiter.remaining_lockout_secs(&ip);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 0);
    }

    #[test]
    fn test_rate_limiter_clear_on_success() {
        let limiter = LoginRateLimiter::new();
        let ip: IpAddr = "192.168.1.2".parse().unwrap();

        // Record some failed attempts
        limiter.record_failed_attempt(ip);
        limiter.record_failed_attempt(ip);

        // Clear on successful login
        limiter.clear_attempts(&ip);

        // Should not be locked out and attempts should be cleared
        assert!(!limiter.is_locked_out(&ip));
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00]), "00");
        assert_eq!(hex_encode(&[0xff]), "ff");
        assert_eq!(hex_encode(&[0x12, 0x34, 0xab]), "1234ab");
        assert_eq!(hex_encode(&[]), "");
    }
}
