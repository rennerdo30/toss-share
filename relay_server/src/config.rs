//! Server configuration

use std::env;

/// Server configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Database URL
    pub database_url: String,
    /// JWT secret for authentication
    pub jwt_secret: String,
    /// JWT token expiration in seconds
    pub jwt_expiration: u64,
    /// Rate limit for relay messages (per minute)
    pub rate_limit_messages: u32,
    /// Rate limit for registration (per hour)
    pub rate_limit_register: u32,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./data/toss.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
                tracing::warn!("JWT_SECRET not set, using random secret");
                generate_random_secret()
            }),
            jwt_expiration: env::var("JWT_EXPIRATION")
                .ok()
                .and_then(|e| e.parse().ok())
                .unwrap_or(86400), // 24 hours
            rate_limit_messages: env::var("RATE_LIMIT_MESSAGES")
                .ok()
                .and_then(|r| r.parse().ok())
                .unwrap_or(100),
            rate_limit_register: env::var("RATE_LIMIT_REGISTER")
                .ok()
                .and_then(|r| r.parse().ok())
                .unwrap_or(10),
        })
    }
}

fn generate_random_secret() -> String {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    let mut rng = StdRng::from_os_rng();
    (0..32)
        .map(|_| rng.random_range(b'a'..=b'z') as char)
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            database_url: "sqlite:./data/toss.db?mode=rwc".to_string(),
            jwt_secret: generate_random_secret(),
            jwt_expiration: 86400,
            rate_limit_messages: 100,
            rate_limit_register: 10,
        }
    }
}
