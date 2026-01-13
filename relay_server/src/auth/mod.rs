//! Authentication and authorization

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{config::Config, error::ApiError, AppState};

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Device ID
    pub sub: String,
    /// Expiration timestamp
    pub exp: u64,
    /// Issued at timestamp
    pub iat: u64,
}

/// Authenticated device extractor
#[derive(Debug, Clone)]
pub struct AuthenticatedDevice {
    pub device_id: String,
}

impl FromRequestParts<AppState> for AuthenticatedDevice {
    type Rejection = ApiError;
 
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let config = Arc::<Config>::from_ref(state);

        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| ApiError::Unauthorized("Missing authorization header".to_string()))?;

        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| ApiError::Unauthorized(format!("Invalid token: {}", e)))?;

        Ok(AuthenticatedDevice {
            device_id: token_data.claims.sub,
        })
    }
}

/// Create a JWT token for a device
pub fn create_token(device_id: &str, secret: &str, expiration_secs: u64) -> Result<String, ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = Claims {
        sub: device_id.to_string(),
        exp: now + expiration_secs,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to create token: {}", e)))
}

/// Verify an Ed25519 signature
pub fn verify_signature(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool, ApiError> {
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| ApiError::BadRequest("Invalid public key length".to_string()))?;

    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| ApiError::BadRequest("Invalid signature length".to_string()))?;

    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|e| ApiError::BadRequest(format!("Invalid public key: {}", e)))?;

    let sig = Signature::from_bytes(&signature);

    Ok(verifying_key.verify(message, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_decode_token() {
        let secret = "test-secret";
        let device_id = "test-device";

        let token = create_token(device_id, secret, 3600).unwrap();

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, device_id);
    }
}
