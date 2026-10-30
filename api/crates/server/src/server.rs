//! Axum HTTP Server Architecture, Tracing & Middlewares

use std::time::Duration;

use axum::{
    http::{HeaderName, StatusCode, Uri},
    middleware::{from_fn, from_fn_with_state},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{errors::ApiError, state::AppState};

/// Canonical correlation header for distributed and server tracing
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Health check response payload
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Handler for the `/health` liveness probe
pub async fn health_check() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            version: roubit_core::VERSION.to_string(),
        }),
    )
}

/// Fallback 404 handler producing RFC 7807 Problem Details
pub async fn fallback_handler(uri: Uri) -> impl IntoResponse {
    ApiError::NotFound(format!("No resource found at {}", uri.path()))
}

/// Constructs the core Axum application router with standard middleware stack
pub fn build_router(state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        );

    let middleware_stack = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ));

    let allowed_origins = state.config.parsed_allowed_origins();
    let cors_layer = crate::middleware::build_cors_layer(&allowed_origins);

    let auth_routes = Router::new()
        .route("/register", post(crate::auth::register))
        .route("/login", post(crate::auth::login))
        .route("/me", get(crate::auth::me));

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/routines", crate::controllers::routine_routes())
        .nest("/habits", crate::controllers::habit_routes())
        .nest("/okrs", crate::controllers::okr_routes())
        .nest("/journal", crate::controllers::journal_routes())
        .layer(from_fn_with_state(
            state.clone(),
            crate::middleware::idempotency_middleware,
        ));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_v1)
        .fallback(fallback_handler)
        .layer(from_fn(crate::middleware::security_headers_middleware))
        .layer(cors_layer)
        .layer(middleware_stack)
        .with_state(state)
}

/// Listens for OS termination signals (SIGINT / SIGTERM) to initiate clean shutdown
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "Failed to install Ctrl+C signal handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to install SIGTERM signal handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        },
        () = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}

/// Binds listener and runs the Axum server with graceful shutdown handling
///
/// # Errors
/// Returns error if binding TCP socket fails or server encountered runtime failure.
pub async fn run_server(
    config: roubit_core::AppConfig,
    pool: sqlx::SqlitePool,
) -> anyhow::Result<()> {
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "Roubit server listening");

    let state = AppState::new(pool, config);
    let app = build_router(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Roubit server stopped cleanly");
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names
)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use roubit_core::{create_sqlite_pool, AppConfig};
    use tower::ServiceExt;

    async fn setup_test_app() -> Router {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("Failed to create memory db");
        roubit_core::run_migrations(&pool)
            .await
            .expect("Failed to apply migrations");
        let config = AppConfig::default();
        let state = AppState::new(pool, config);
        build_router(state)
    }

    #[tokio::test]
    async fn test_health_check_returns_ok_and_request_id() {
        let app = setup_test_app().await;

        let request = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");
        assert_eq!(response.status(), StatusCode::OK);

        // Verify x-request-id header exists and is valid UUID
        let req_id_header = response.headers().get("x-request-id");
        assert!(
            req_id_header.is_some(),
            "x-request-id header must be present"
        );
        let req_id_str = req_id_header.unwrap().to_str().expect("Valid UTF-8 header");
        assert_ne!(req_id_str, "");
        assert!(
            uuid::Uuid::parse_str(req_id_str).is_ok(),
            "Request ID should be a valid UUID"
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let health: HealthResponse = serde_json::from_slice(&body).expect("Failed to parse json");
        assert_eq!(health.status, "ok");
        assert_eq!(health.version, roubit_core::VERSION);
    }

    #[tokio::test]
    async fn test_request_id_propagation() {
        let app = setup_test_app().await;

        let custom_id = "test-custom-request-id-123";
        let request = Request::builder()
            .uri("/health")
            .method("GET")
            .header("x-request-id", custom_id)
            .body(Body::empty())
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");
        assert_eq!(response.status(), StatusCode::OK);

        let echoed_id = response
            .headers()
            .get("x-request-id")
            .expect("Header must be present")
            .to_str()
            .expect("Valid UTF-8");
        assert_eq!(echoed_id, custom_id);
    }

    #[tokio::test]
    async fn test_fallback_returns_rfc7807_problem_details() {
        let app = setup_test_app().await;

        let request = Request::builder()
            .uri("/non-existent-endpoint")
            .method("GET")
            .body(Body::empty())
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: crate::errors::ProblemDetails =
            serde_json::from_slice(&body).expect("Valid RFC 7807 JSON");

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert_eq!(
            problem.detail,
            "No resource found at /non-existent-endpoint"
        );
        assert_eq!(problem.r#type, "https://roubit.app/errors/404");
    }

    #[tokio::test]
    async fn test_owasp_security_headers_present() {
        let app = setup_test_app().await;

        let request = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");

        let headers = response.headers();
        assert_eq!(
            headers
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
            Some("DENY")
        );
        assert_eq!(
            headers
                .get("strict-transport-security")
                .and_then(|v| v.to_str().ok()),
            Some("max-age=63072000; includeSubDomains")
        );
        assert_eq!(
            headers
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok()),
            Some("default-src 'self'")
        );
        assert_eq!(
            headers.get("referrer-policy").and_then(|v| v.to_str().ok()),
            Some("strict-origin-when-cross-origin")
        );
    }

    #[tokio::test]
    async fn test_auth_register_validation_failures() {
        let app = setup_test_app().await;

        // Invalid email
        let req_invalid_email = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email": "notanemail", "password": "securepassword123"}"#,
            ))
            .expect("Failed to build request");

        let res = app
            .clone()
            .oneshot(req_invalid_email)
            .await
            .expect("Failed to execute");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        // Short password (< 8 chars)
        let req_short_pw = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email": "test@roubit.app", "password": "short"}"#,
            ))
            .expect("Failed to build request");

        let res = app.oneshot(req_short_pw).await.expect("Failed to execute");
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_auth_register_login_and_me_full_flow() {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("Failed to create memory db");
        roubit_core::run_migrations(&pool)
            .await
            .expect("Failed to apply migrations");
        let config = AppConfig::default();
        let state = AppState::new(pool, config);

        // 1. Register new user
        let reg_payload = serde_json::to_string(&crate::auth::RegisterRequest {
            email: "alice@roubit.app".to_string(),
            password: "supersecurepassword123".to_string(),
        })
        .expect("Valid JSON");

        let reg_req = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(reg_payload.clone()))
            .expect("Failed to build request");

        let reg_res = build_router(state.clone())
            .oneshot(reg_req)
            .await
            .expect("Failed to execute");
        assert_eq!(reg_res.status(), StatusCode::CREATED);

        let cookie_header = reg_res.headers().get("set-cookie");
        assert!(cookie_header.is_some());
        let cookie_str = cookie_header.unwrap().to_str().expect("Valid UTF-8");
        assert!(cookie_str.contains("roubit_token="));
        assert!(cookie_str.contains("HttpOnly"));
        assert!(cookie_str.contains("SameSite=Strict"));

        let reg_body = to_bytes(reg_res.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let auth_res: crate::auth::AuthResponse =
            serde_json::from_slice(&reg_body).expect("Valid AuthResponse JSON");
        assert_eq!(auth_res.user.email, "alice@roubit.app");
        assert_ne!(auth_res.token, "");

        // 2. Duplicate registration -> 409 Conflict
        let dup_req = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(reg_payload))
            .expect("Failed to build request");

        let dup_res = build_router(state.clone())
            .oneshot(dup_req)
            .await
            .expect("Failed to execute");
        assert_eq!(dup_res.status(), StatusCode::CONFLICT);

        // 3. Login with incorrect password -> 401 Unauthorized
        let wrong_login_req = Request::builder()
            .uri("/api/v1/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email": "alice@roubit.app", "password": "wrongpassword!"}"#,
            ))
            .expect("Failed to build request");

        let wrong_login_res = build_router(state.clone())
            .oneshot(wrong_login_req)
            .await
            .expect("Failed to execute");
        assert_eq!(wrong_login_res.status(), StatusCode::UNAUTHORIZED);

        // 4. Login with correct password -> 200 OK
        let login_req = Request::builder()
            .uri("/api/v1/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email": "alice@roubit.app", "password": "supersecurepassword123"}"#,
            ))
            .expect("Failed to build request");

        let login_res = build_router(state.clone())
            .oneshot(login_req)
            .await
            .expect("Failed to execute");
        assert_eq!(login_res.status(), StatusCode::OK);

        // 5. Access /api/v1/auth/me without token -> 401 Unauthorized
        let me_unauth_req = Request::builder()
            .uri("/api/v1/auth/me")
            .method("GET")
            .body(Body::empty())
            .expect("Failed to build request");

        let me_unauth_res = build_router(state.clone())
            .oneshot(me_unauth_req)
            .await
            .expect("Failed to execute");
        assert_eq!(me_unauth_res.status(), StatusCode::UNAUTHORIZED);

        // 6. Access /api/v1/auth/me with Bearer token -> 200 OK
        let me_auth_req = Request::builder()
            .uri("/api/v1/auth/me")
            .method("GET")
            .header("authorization", format!("Bearer {}", auth_res.token))
            .body(Body::empty())
            .expect("Failed to build request");

        let me_auth_res = build_router(state.clone())
            .oneshot(me_auth_req)
            .await
            .expect("Failed to execute");
        assert_eq!(me_auth_res.status(), StatusCode::OK);

        let me_body = to_bytes(me_auth_res.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let me_user: crate::auth::UserDto =
            serde_json::from_slice(&me_body).expect("Valid UserDto JSON");
        assert_eq!(me_user.email, "alice@roubit.app");
        assert_eq!(me_user.id, auth_res.user.id);

        // 7. Access /api/v1/auth/me with Cookie -> 200 OK
        let me_cookie_req = Request::builder()
            .uri("/api/v1/auth/me")
            .method("GET")
            .header("cookie", format!("roubit_token={}", auth_res.token))
            .body(Body::empty())
            .expect("Failed to build request");

        let me_cookie_res = build_router(state.clone())
            .oneshot(me_cookie_req)
            .await
            .expect("Failed to execute");
        assert_eq!(me_cookie_res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_idempotency_key_replay_and_mismatch() {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("Failed to create memory db");
        roubit_core::run_migrations(&pool)
            .await
            .expect("Failed to apply migrations");
        let config = AppConfig::default();
        let state = AppState::new(pool, config);

        let idempotency_key = "test-key-uuid-123456";
        let payload1 = serde_json::to_string(&crate::auth::RegisterRequest {
            email: "idempotent_user@roubit.app".to_string(),
            password: "supersecurepassword123".to_string(),
        })
        .expect("Valid JSON");

        // First call: initial execution -> 201 Created, X-Cache: MISS
        let req1 = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .header("idempotency-key", idempotency_key)
            .body(Body::from(payload1.clone()))
            .expect("Failed to build request");

        let res1 = build_router(state.clone())
            .oneshot(req1)
            .await
            .expect("Failed to execute");
        assert_eq!(res1.status(), StatusCode::CREATED);
        assert_eq!(
            res1.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("MISS")
        );

        let body1 = to_bytes(res1.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let auth1: crate::auth::AuthResponse = serde_json::from_slice(&body1).expect("Valid JSON");

        // Second call with EXACT SAME key and payload -> returns cached response (201 Created, X-Cache: HIT)
        let req2 = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .header("idempotency-key", idempotency_key)
            .body(Body::from(payload1))
            .expect("Failed to build request");

        let res2 = build_router(state.clone())
            .oneshot(req2)
            .await
            .expect("Failed to execute");
        assert_eq!(res2.status(), StatusCode::CREATED);
        assert_eq!(
            res2.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("HIT")
        );

        let body2 = to_bytes(res2.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let auth2: crate::auth::AuthResponse = serde_json::from_slice(&body2).expect("Valid JSON");
        assert_eq!(auth1, auth2);

        // Third call with SAME key but DIFFERENT payload -> 409 Conflict
        let payload_different = serde_json::to_string(&crate::auth::RegisterRequest {
            email: "different_user@roubit.app".to_string(),
            password: "anotherpassword123".to_string(),
        })
        .expect("Valid JSON");

        let req3 = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .header("idempotency-key", idempotency_key)
            .body(Body::from(payload_different))
            .expect("Failed to build request");

        let res3 = build_router(state.clone())
            .oneshot(req3)
            .await
            .expect("Failed to execute");
        assert_eq!(res3.status(), StatusCode::CONFLICT);

        // Fourth call with key > 64 chars -> 400 Bad Request
        let long_key = "a".repeat(65);
        let req_long_key = Request::builder()
            .uri("/api/v1/auth/register")
            .method("POST")
            .header("content-type", "application/json")
            .header("idempotency-key", long_key)
            .body(Body::from("{}"))
            .expect("Failed to build request");

        let res_long = build_router(state)
            .oneshot(req_long_key)
            .await
            .expect("Failed to execute");
        assert_eq!(res_long.status(), StatusCode::BAD_REQUEST);
    }
}
