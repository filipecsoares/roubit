//! JWT token generation, validation, and Argon2id password hashing

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::ApiError;

/// JWT token payload claims
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// Subject identifier (User UUID)
    pub sub: Uuid,
    /// User's email address
    pub email: String,
    /// Token expiration timestamp in seconds (UNIX epoch)
    pub exp: usize,
    /// Token issuance timestamp in seconds (UNIX epoch)
    pub iat: usize,
}

/// Hashes a plaintext password using Argon2id with cryptographically secure random salt
///
/// # Errors
/// Returns [`ApiError::Internal`] if hashing fails.
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Password hashing failed: {e}")))
}

/// Verifies a password against an Argon2id hash using constant-time comparison
///
/// # Errors
/// Returns [`ApiError::Internal`] if the hash string is malformed.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, ApiError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Invalid password hash format: {e}")))?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Encodes a signed JWT with HMAC-SHA256 (HS256)
///
/// # Errors
/// Returns [`ApiError::Internal`] if token signing fails.
pub fn encode_jwt(
    user_id: Uuid,
    email: &str,
    secret: &str,
    expiration_seconds: u64,
) -> Result<String, ApiError> {
    let now = chrono::Utc::now().timestamp();
    let exp_timestamp = now.saturating_add(i64::try_from(expiration_seconds).unwrap_or(3600));

    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        exp: usize::try_from(exp_timestamp).unwrap_or(0),
        iat: usize::try_from(now).unwrap_or(0),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to sign JWT: {e}")))
}

/// Decodes and validates a signed JWT
///
/// # Errors
/// Returns [`ApiError::Unauthorized`] if token signature or expiration is invalid.
pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, ApiError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Unauthorized("Invalid or expired authentication token".to_string()))?;

    Ok(token_data.claims)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2id_hash_and_verify_success() {
        let password = "SuperSecretPassword123!";
        let hash = hash_password(password).expect("Hashing should succeed");
        assert!(hash.starts_with("$argon2id$"));

        let is_valid = verify_password(password, &hash).expect("Verification should succeed");
        assert!(is_valid);
    }

    #[test]
    fn test_argon2id_wrong_password_rejected() {
        let hash = hash_password("CorrectPassword").expect("Hashing should succeed");
        let is_valid =
            verify_password("WrongPassword", &hash).expect("Verification should succeed");
        assert!(!is_valid);
    }

    #[test]
    fn test_jwt_encode_decode_roundtrip() {
        let user_id = Uuid::new_v4();
        let email = "user@roubit.app";
        let secret = "top-secret-signing-key-for-roubit-test-suite";

        let token = encode_jwt(user_id, email, secret, 3600).expect("Encoding should succeed");
        assert_ne!(token, "");

        let claims = decode_jwt(&token, secret).expect("Decoding should succeed");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_jwt_tampered_signature_rejected() {
        let user_id = Uuid::new_v4();
        let email = "user@roubit.app";
        let secret = "top-secret-signing-key-for-roubit-test-suite";

        let token = encode_jwt(user_id, email, secret, 3600).expect("Encoding should succeed");
        let wrong_secret = "different-secret-key-that-fails-verification";

        let result = decode_jwt(&token, wrong_secret);
        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::Unauthorized(msg) => {
                assert!(msg.contains("Invalid or expired"));
            }
            other => panic!("Expected Unauthorized error, got {other:?}"),
        }
    }
}
