//! Axum HTTP Server Architecture, Tracing & Middlewares

use std::time::Duration;

use axum::{
    http::{HeaderName, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
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

    Router::new()
        .route("/health", get(health_check))
        .fallback(fallback_handler)
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
}
