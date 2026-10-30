//! Transactional Idempotency Middleware (RF05)
//!
//! Intercepts state-mutating HTTP methods (POST, PUT, PATCH) carrying an `Idempotency-Key`
//! header to ensure at-most-once execution. Computes SHA-256 payload fingerprints to detect
//! payload mismatches, locks in-flight requests against concurrent execution, and caches/replays
//! responses with `X-Cache: HIT`.

use std::collections::HashMap;

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::{
    auth::jwt::decode_jwt, errors::ApiError, middleware::security::IDEMPOTENCY_KEY_HEADER,
    state::AppState,
};

/// Header indicating whether response was served from cache
pub const X_CACHE_HEADER: HeaderName = HeaderName::from_static("x-cache");

const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 64;
const IN_FLIGHT_LOCK_DURATION_SECONDS: i64 = 30;

fn extract_user_id(parts: &axum::http::request::Parts, jwt_secret: &str) -> String {
    // 1. Check Authorization: Bearer <token>
    if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(claims) = decode_jwt(token.trim(), jwt_secret) {
                    return claims.sub.to_string();
                }
            }
        }
    }

    // 2. Check Cookie: roubit_token=<token>
    if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for pair in cookie_str.split(';') {
                let mut parts_split = pair.splitn(2, '=');
                if let (Some(key), Some(val)) = (parts_split.next(), parts_split.next()) {
                    if key.trim() == "roubit_token" {
                        if let Ok(claims) = decode_jwt(val.trim(), jwt_secret) {
                            return claims.sub.to_string();
                        }
                    }
                }
            }
        }
    }

    "anonymous".to_string()
}

/// Idempotency middleware for transactional state mutation endpoints
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on invalid key, [`ApiError::Conflict`] on payload mismatch
/// or concurrent execution, or [`ApiError::Internal`] on database failure.
#[allow(clippy::too_many_lines)]
pub async fn idempotency_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Only intercept state-mutating HTTP methods
    if req.method() != Method::POST && req.method() != Method::PUT && req.method() != Method::PATCH
    {
        return Ok(next.run(req).await);
    }

    // Check for Idempotency-Key header
    let Some(key_header) = req.headers().get(&IDEMPOTENCY_KEY_HEADER) else {
        return Ok(next.run(req).await);
    };

    let key = key_header
        .to_str()
        .map_err(|_| ApiError::BadRequest("Invalid Idempotency-Key header encoding".to_string()))?
        .trim()
        .to_string();

    if key.is_empty() || key.len() > MAX_IDEMPOTENCY_KEY_LENGTH {
        return Err(ApiError::BadRequest(format!(
            "Idempotency-Key must be non-empty and not exceed {MAX_IDEMPOTENCY_KEY_LENGTH} characters"
        )));
    }

    let (parts, body) = req.into_parts();
    let user_id = extract_user_id(&parts, &state.config.jwt_secret);
    let request_path = parts.uri.path().to_string();

    // Read body bytes and compute SHA-256 fingerprint
    let body_bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to read request body: {e}")))?;
    let request_hash = Sha256::digest(&body_bytes).to_vec();

    let now = chrono::Utc::now();
    let locked_until = now + chrono::Duration::seconds(IN_FLIGHT_LOCK_DURATION_SECONDS);

    // Query existing key state for this scoped user
    let existing_row = sqlx::query(
        r"
        SELECT response_status, response_headers, response_body, request_hash, locked_until
        FROM idempotency_keys
        WHERE user_id = ? AND key = ?
        ",
    )
    .bind(&user_id)
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some(row) = existing_row {
        let stored_hash: Vec<u8> = row.get("request_hash");
        let response_status_opt: Option<i64> = row.get("response_status");
        let locked_until_str: String = row.get("locked_until");

        // Payload fingerprint verification
        if stored_hash != request_hash {
            return Err(ApiError::Conflict(
                "Idempotency key payload mismatch: request body differs from original request"
                    .to_string(),
            ));
        }

        // Case 1: Request already completed -> Replay cached response
        if let Some(status_code) = response_status_opt {
            let u16_status = u16::try_from(status_code)
                .map_err(|_| ApiError::Internal(anyhow::anyhow!("Invalid status code in cache")))?;
            let status =
                StatusCode::from_u16(u16_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            let headers_json: Option<String> = row.get("response_headers");
            let cached_body: Vec<u8> = row.get("response_body");

            let mut response = Response::builder()
                .status(status)
                .header(X_CACHE_HEADER, HeaderValue::from_static("HIT"));

            if let Some(json_str) = headers_json {
                if let Ok(headers_map) = serde_json::from_str::<HashMap<String, String>>(&json_str)
                {
                    for (header_name, header_val) in headers_map {
                        if let (Ok(h_name), Ok(h_val)) = (
                            HeaderName::from_bytes(header_name.as_bytes()),
                            HeaderValue::from_str(&header_val),
                        ) {
                            response = response.header(h_name, h_val);
                        }
                    }
                }
            }

            return response
                .body(Body::from(cached_body))
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to build response: {e}")));
        }

        // Case 2: In-flight request currently running and lock active
        let locked_until_time = chrono::DateTime::parse_from_rfc3339(&locked_until_str)
            .map_or(now, |dt| dt.with_timezone(&chrono::Utc));

        if locked_until_time > now {
            return Err(ApiError::Conflict(
                "A request with this idempotency key is currently being processed".to_string(),
            ));
        }

        // Case 3: Expired lock without completion (worker timed out) -> Re-acquire lock
        sqlx::query(
            r"
            UPDATE idempotency_keys
            SET locked_until = ?, request_hash = ?, request_path = ?
            WHERE user_id = ? AND key = ?
            ",
        )
        .bind(locked_until.to_rfc3339())
        .bind(&request_hash)
        .bind(&request_path)
        .bind(&user_id)
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    } else {
        // Insert new in-flight record
        sqlx::query(
            r"
            INSERT INTO idempotency_keys
                (key, user_id, request_path, request_hash, response_status, response_headers, response_body, created_at, locked_until)
            VALUES (?, ?, ?, ?, NULL, NULL, NULL, ?, ?)
            ",
        )
        .bind(&key)
        .bind(&user_id)
        .bind(&request_path)
        .bind(&request_hash)
        .bind(now.to_rfc3339())
        .bind(locked_until.to_rfc3339())
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    // Reconstruct request with buffered body bytes
    let reconstructed_req = Request::from_parts(parts, Body::from(body_bytes));
    let downstream_response = next.run(reconstructed_req).await;

    let (res_parts, res_body) = downstream_response.into_parts();
    let res_bytes = to_bytes(res_body, usize::MAX)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Failed to read response body: {e}")))?;

    // Store response in cache if not a 5xx server error
    if res_parts.status.as_u16() < 500 {
        let mut headers_to_cache = HashMap::new();
        for (h_name, h_val) in &res_parts.headers {
            if let Ok(val_str) = h_val.to_str() {
                headers_to_cache.insert(h_name.to_string(), val_str.to_string());
            }
        }
        let headers_json = serde_json::to_string(&headers_to_cache).unwrap_or_default();

        let status_code_i64 = i64::from(res_parts.status.as_u16());
        let res_slice: &[u8] = &res_bytes;

        sqlx::query(
            r"
            UPDATE idempotency_keys
            SET response_status = ?, response_headers = ?, response_body = ?
            WHERE user_id = ? AND key = ?
            ",
        )
        .bind(status_code_i64)
        .bind(&headers_json)
        .bind(res_slice)
        .bind(&user_id)
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    }

    let mut response = Response::from_parts(res_parts, Body::from(res_bytes));
    response
        .headers_mut()
        .insert(X_CACHE_HEADER, HeaderValue::from_static("MISS"));

    Ok(response)
}

/// Purges idempotency keys older than the specified duration to conserve storage
///
/// # Errors
/// Returns [`sqlx::Error`] if cleanup query fails.
pub async fn purge_expired_keys(
    pool: &sqlx::SqlitePool,
    max_age: chrono::Duration,
) -> Result<u64, sqlx::Error> {
    let cutoff = (chrono::Utc::now() - max_age).to_rfc3339();
    let result = sqlx::query("DELETE FROM idempotency_keys WHERE created_at < ?")
        .bind(&cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use roubit_core::create_sqlite_pool;

    #[tokio::test]
    async fn test_purge_expired_keys() {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("Failed to create memory db");
        roubit_core::run_migrations(&pool)
            .await
            .expect("Failed to apply migrations");

        let old_time = (chrono::Utc::now() - chrono::Duration::hours(72)).to_rfc3339();
        let recent_time = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r"
            INSERT INTO idempotency_keys (key, user_id, request_path, request_hash, created_at, locked_until)
            VALUES ('old_key', 'user1', '/api/test', X'1234', ?, ?)
            ",
        )
        .bind(&old_time)
        .bind(&old_time)
        .execute(&pool)
        .await
        .expect("Failed to insert old key");

        sqlx::query(
            r"
            INSERT INTO idempotency_keys (key, user_id, request_path, request_hash, created_at, locked_until)
            VALUES ('new_key', 'user1', '/api/test', X'1234', ?, ?)
            ",
        )
        .bind(&recent_time)
        .bind(&recent_time)
        .execute(&pool)
        .await
        .expect("Failed to insert new key");

        let purged = purge_expired_keys(&pool, chrono::Duration::hours(48))
            .await
            .expect("Failed to purge keys");
        assert_eq!(purged, 1);

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM idempotency_keys")
            .fetch_one(&pool)
            .await
            .expect("Failed to count");
        assert_eq!(remaining, 1);
    }
}
