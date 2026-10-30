#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use roubit_core::{create_sqlite_pool, AppConfig};
use roubit_server::{build_router, AppState, HealthResponse, REQUEST_ID_HEADER};

#[tokio::test]
async fn test_tcp_server_startup_and_health_endpoint() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");
    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        ..Default::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral port");
    let local_addr = listener.local_addr().expect("Failed to get local address");

    let state = AppState::new(pool, config);
    let app = build_router(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("Server exited with error");
    });

    let client = reqwest::Client::new();
    let health_url = format!("http://{local_addr}/health");

    let response = client
        .get(&health_url)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let req_id = response
        .headers()
        .get(REQUEST_ID_HEADER.as_str())
        .expect("Missing x-request-id")
        .to_str()
        .expect("Valid UTF-8");
    assert!(uuid::Uuid::parse_str(req_id).is_ok());

    let body: HealthResponse = response.json().await.expect("Valid health json");
    assert_eq!(body.status, "ok");
    assert_eq!(body.version, roubit_core::VERSION);

    // Test 404 on TCP server
    let not_found_url = format!("http://{local_addr}/does-not-exist");
    let not_found_res = client
        .get(&not_found_url)
        .send()
        .await
        .expect("Failed to send 404 request");
    assert_eq!(not_found_res.status(), reqwest::StatusCode::NOT_FOUND);

    let problem: roubit_server::ProblemDetails = not_found_res
        .json()
        .await
        .expect("Failed to parse ProblemDetails");
    assert_eq!(problem.status, 404);
    assert_eq!(problem.title, "Not Found");

    // Trigger graceful shutdown
    let _ = shutdown_tx.send(());

    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Server did not shut down cleanly in time")
        .expect("Server task panicked");
}

#[tokio::test]
async fn test_tcp_auth_register_and_protected_me_endpoint() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");
    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        ..Default::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral port");
    let local_addr = listener.local_addr().expect("Failed to get local address");

    let state = AppState::new(pool, config);
    let app = build_router(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("Server exited with error");
    });

    let client = reqwest::Client::new();
    let reg_url = format!("http://{local_addr}/api/v1/auth/register");

    let reg_res = client
        .post(&reg_url)
        .json(&roubit_server::RegisterRequest {
            email: "bob@roubit.app".to_string(),
            password: "supersecurepassword123".to_string(),
        })
        .send()
        .await
        .expect("Register request failed");

    assert_eq!(reg_res.status(), reqwest::StatusCode::CREATED);
    assert_eq!(
        reg_res
            .headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );

    let auth_body: roubit_server::AuthResponse =
        reg_res.json().await.expect("Failed to parse AuthResponse");
    assert_eq!(auth_body.user.email, "bob@roubit.app");

    // Call /api/v1/auth/me with Bearer token
    let me_url = format!("http://{local_addr}/api/v1/auth/me");
    let me_res = client
        .get(&me_url)
        .bearer_auth(&auth_body.token)
        .send()
        .await
        .expect("Me request failed");

    assert_eq!(me_res.status(), reqwest::StatusCode::OK);
    let me_body: roubit_server::UserDto = me_res.json().await.expect("Failed to parse UserDto");
    assert_eq!(me_body.email, "bob@roubit.app");
    assert_eq!(me_body.id, auth_body.user.id);

    // Call /api/v1/auth/me without token -> 401
    let unauth_res = client.get(&me_url).send().await.expect("Request failed");
    assert_eq!(unauth_res.status(), reqwest::StatusCode::UNAUTHORIZED);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}
