//! Admin authentication for the dashboard

use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

use crate::AppState;

const SESSION_COOKIE_NAME: &str = "admin_session";

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

/// Display login page
pub async fn login_page(State(state): State<AppState>) -> impl IntoResponse {
    // If admin is not enabled, return 404
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
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
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if !state.config.admin_enabled() {
        return (StatusCode::NOT_FOUND, "Admin dashboard not configured").into_response();
    }

    let expected_username = state.config.admin_username.as_deref().unwrap_or("");
    let password_hash = state.config.admin_password_hash.as_deref().unwrap_or("");

    // Verify credentials
    let username_matches = form.username == expected_username;
    let password_matches = bcrypt::verify(&form.password, password_hash).unwrap_or(false);

    if username_matches && password_matches {
        // Create session token
        let session_token = create_session_token(&state.config.session_secret);

        // Set cookie and redirect to dashboard
        let cookie = format!(
            "{}={}; Path=/admin; HttpOnly; SameSite=Strict; Max-Age=86400",
            SESSION_COOKIE_NAME, session_token
        );

        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(header::SET_COOKIE, cookie)
            .header(header::LOCATION, "/admin")
            .body(axum::body::Body::empty())
            .unwrap()
            .into_response()
    } else {
        let template = LoginTemplate {
            error: "Invalid username or password".to_string(),
            has_error: true,
        };
        Html(template.render().unwrap_or_default()).into_response()
    }
}

/// Handle logout
pub async fn logout() -> impl IntoResponse {
    // Clear the session cookie
    let cookie = format!(
        "{}=; Path=/admin; HttpOnly; SameSite=Strict; Max-Age=0",
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

/// Create a simple session token (HMAC-based)
fn create_session_token(secret: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simple token: timestamp:signature
    let data = format!("admin:{}", timestamp);
    let signature = simple_hmac(secret, &data);

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

    // Check if token is not too old (24 hours)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now - timestamp > 86400 {
        return false;
    }

    // Verify signature
    let data = format!("admin:{}", timestamp);
    let expected_signature = simple_hmac(secret, &data);

    parts[1] == expected_signature
}

/// Simple HMAC-like signature (for session tokens)
fn simple_hmac(secret: &str, data: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
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
