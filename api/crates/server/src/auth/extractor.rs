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
        let cookie_token_opt = if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
            let cookie_str = cookie_header.to_str().map_err(|_| {
                ApiError::Unauthorized("Invalid cookie header encoding".to_string())
            })?;

            cookie_str.split(';').find_map(|pair| {
                let mut p = pair.splitn(2, '=');
                let key = p.next()?.trim();
                let val = p.next()?.trim();
                if key == "roubit_token" {
                    Some(val.to_string())
                } else {
                    None
                }
            })
        } else {
            None
        };

        // 3. Fallback to URL query parameter (?token=... or ?access_token=...) strictly for SSE EventSource
        // Browser EventSource API does not support custom Authorization headers.
        // For standard REST endpoints, query tokens are rejected to prevent leakage in URL logs/referrers.
        let query_token_opt = if parts.uri.path().ends_with("/notifications/stream") {
            parts.uri.query().and_then(|query| {
                query.split('&').find_map(|pair| {
                    let mut p = pair.splitn(2, '=');
                    let key = p.next()?.trim();
                    let val = p.next()?.trim();
                    if key == "token" || key == "access_token" {
                        Some(percent_decode(val))
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        };

        let token = token_opt
            .or(cookie_token_opt)
            .or(query_token_opt)
            .ok_or_else(|| ApiError::Unauthorized("Missing authentication token".to_string()))?;

        let claims = decode_jwt(&token, &app_state.config.jwt_secret)?;
        Ok(Self {
            id: claims.sub,
            email: claims.email,
        })
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..=i + 2]).unwrap_or(""), 16)
            {
                decoded.push(hex_val);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            decoded.push(b' ');
            i += 1;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| s.to_string())
}
