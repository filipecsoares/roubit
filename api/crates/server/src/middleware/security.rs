use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use axum::{
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::errors::ProblemDetails;
use crate::server::REQUEST_ID_HEADER;
use crate::state::AppState;

/// Canonical `HeaderName` for transaction idempotency
pub const IDEMPOTENCY_KEY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");

/// In-memory token bucket rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    auth_per_minute: u32,
    general_per_minute: u32,
    buckets: Mutex<HashMap<String, (f64, Instant)>>,
}

impl RateLimiter {
    /// Creates a new rate limiter with specified limits per minute.
    #[must_use]
    pub fn new(auth_per_minute: u32, general_per_minute: u32) -> Self {
        Self {
            auth_per_minute,
            general_per_minute,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if a request is allowed for the given key and limit.
    ///
    /// # Errors
    /// Returns `Err(retry_after_seconds)` when the rate limit has been exceeded.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn check(&self, key: &str, limit_per_minute: u32) -> Result<(), u64> {
        let capacity = f64::from(limit_per_minute);
        let rate_per_sec = capacity / 60.0;
        let now = Instant::now();

        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Garbage collection when map gets large to prevent unbounded memory growth
        if buckets.len() > 10_000 {
            buckets.retain(|_, (_, last_refreshed)| {
                now.duration_since(*last_refreshed).as_secs() < 300
            });
        }

        let entry = buckets
            .entry(key.to_string())
            .or_insert_with(|| (capacity, now));
        let elapsed = now.duration_since(entry.1).as_secs_f64();
        entry.0 = elapsed.mul_add(rate_per_sec, entry.0).min(capacity);
        entry.1 = now;

        let result = if entry.0 >= 1.0 {
            entry.0 -= 1.0;
            Ok(())
        } else {
            let needed = 1.0 - entry.0;
            let retry_after = (needed / rate_per_sec).ceil() as u64;
            Err(retry_after.max(1))
        };
        drop(buckets);
        result
    }

    /// Number of allowed requests per minute for auth routes
    #[must_use]
    pub const fn auth_limit(&self) -> u32 {
        self.auth_per_minute
    }

    /// Number of allowed requests per minute for general API routes
    #[must_use]
    pub const fn general_limit(&self) -> u32 {
        self.general_per_minute
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(20, 300)
    }
}

fn extract_client_identifier(req: &Request, config_jwt_secret: &str) -> String {
    // 1. Try Authorization: Bearer <jwt>
    if let Some(auth_val) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(token) = auth_val.strip_prefix("Bearer ") {
            if let Ok(claims) = crate::auth::decode_jwt(token.trim(), config_jwt_secret) {
                return format!("user:{}", claims.sub);
            }
        }
    }

    // 2. Try Cookie: roubit_token=<jwt>
    if let Some(cookie_hdr) = req
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
    {
        for pair in cookie_hdr.split(';') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                if k.trim() == "roubit_token" {
                    if let Ok(claims) = crate::auth::decode_jwt(v.trim(), config_jwt_secret) {
                        return format!("user:{}", claims.sub);
                    }
                }
            }
        }
    }

    // 3. Fallback to client IP from X-Forwarded-For or X-Real-IP
    if let Some(xff) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        if let Some(first_ip) = xff.split(',').next() {
            let trimmed = first_ip.trim();
            if !trimmed.is_empty() {
                return format!("ip:{trimmed}");
            }
        }
    }

    if let Some(real_ip) = req.headers().get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let trimmed = real_ip.trim();
        if !trimmed.is_empty() {
            return format!("ip:{trimmed}");
        }
    }

    "ip:127.0.0.1".to_string()
}

/// Rate limiting middleware protecting auth and general API routes against brute force and `DoS`
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.rate_limit_enabled {
        return next.run(request).await;
    }

    let path = request.uri().path();
    // Bypass health and readiness probes
    if path == "/health" || path == "/ready" {
        return next.run(request).await;
    }

    let is_auth_route = path.starts_with("/api/v1/auth") || path.starts_with("/auth");
    let (prefix, limit) = if is_auth_route {
        ("auth", state.rate_limiter.auth_limit())
    } else {
        ("gen", state.rate_limiter.general_limit())
    };

    let client_id = extract_client_identifier(&request, &state.config.jwt_secret);
    let key = format!("{prefix}:{client_id}");

    match state.rate_limiter.check(&key, limit) {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            let problem = ProblemDetails {
                r#type: "https://roubit.app/errors/429".to_string(),
                title: "Too Many Requests".to_string(),
                status: 429,
                detail: format!("Rate limit exceeded. Try again in {retry_after} seconds."),
                instance: None,
            };
            let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(problem)).into_response();
            if let Ok(val) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, val);
            }
            response
        }
    }
}

/// Middleware injecting OWASP recommended security headers onto every response
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
}

/// Builds a hardened CORS layer restricting origins to specified allowed domains
pub fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            IDEMPOTENCY_KEY_HEADER,
            REQUEST_ID_HEADER,
        ])
        .allow_credentials(true)
}
