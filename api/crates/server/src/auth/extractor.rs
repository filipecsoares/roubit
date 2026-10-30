//! Authenticated user extractor for Axum route protection

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::{auth::jwt::decode_jwt, errors::ApiError, state::AppState};

/// Extracted context of a cryptographically validated user
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    /// Unique user UUID
    pub id: Uuid,
    /// Verified email address
    pub email: String,
}

#[allow(clippy::unused_async_trait_impl)]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // 1. Attempt extracting from Authorization: Bearer <token>
        let token_opt = if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
            let auth_str = auth_header.to_str().map_err(|_| {
                ApiError::Unauthorized("Invalid authorization header encoding".to_string())
            })?;

            auth_str
                .strip_prefix("Bearer ")
                .map(|t| t.trim().to_string())
        } else {
            None
        };

        // 2. Fallback to HttpOnly cookie: roubit_token=<token>
        let token = if let Some(t) = token_opt {
            t
        } else if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
            let cookie_str = cookie_header.to_str().map_err(|_| {
                ApiError::Unauthorized("Invalid cookie header encoding".to_string())
            })?;

            cookie_str
                .split(';')
                .find_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?.trim();
                    let val = parts.next()?.trim();
                    if key == "roubit_token" {
                        Some(val.to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| ApiError::Unauthorized("Missing authentication token".to_string()))?
        } else {
            return Err(ApiError::Unauthorized(
                "Missing authentication token".to_string(),
            ));
        };

        let claims = decode_jwt(&token, &app_state.config.jwt_secret)?;
        Ok(Self {
            id: claims.sub,
            email: claims.email,
        })
    }
}
