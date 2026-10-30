//! OWASP Security Headers and CORS Policy Hardening

use axum::{
    extract::Request,
    http::{header, HeaderName, HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::server::REQUEST_ID_HEADER;

/// Canonical `HeaderName` for transaction idempotency
pub const IDEMPOTENCY_KEY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");

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
