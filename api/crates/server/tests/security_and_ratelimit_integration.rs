#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use roubit_core::{create_sqlite_pool, AppConfig};
use roubit_server::{build_router, AppState, ProblemDetails};

#[tokio::test]
async fn test_rate_limiter_auth_burst_exceeded_returns_429() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");

    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "rate_limiter_test_secret_32_bytes_long!".to_string(),
        auth_rate_limit: 5, // low limit for quick burst testing
        general_rate_limit: 100,
        rate_limit_enabled: true,
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
            .expect("Server error");
    });

    let client = reqwest::Client::new();
    let login_url = format!("http://{local_addr}/api/v1/auth/login");

    // Send 5 requests within burst quota
    for i in 1..=5 {
        let res = client
            .post(&login_url)
            .json(&serde_json::json!({
                "email": format!("user{i}@roubit.app"),
                "password": "wrongpassword123"
            }))
            .send()
            .await
            .expect("Send request");
        assert_ne!(
            res.status(),
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "Request {i} within burst limit should not be 429"
        );
    }

    // 6th request must trigger 429 Too Many Requests
    let rate_limited_res = client
        .post(&login_url)
        .json(&serde_json::json!({
            "email": "user6@roubit.app",
            "password": "wrongpassword123"
        }))
        .send()
        .await
        .expect("Send request");

    assert_eq!(
        rate_limited_res.status(),
        reqwest::StatusCode::TOO_MANY_REQUESTS
    );

    // Verify Retry-After header
    assert!(rate_limited_res.headers().contains_key("retry-after"));

    // Verify RFC 7807 ProblemDetails format
    let problem: ProblemDetails = rate_limited_res
        .json()
        .await
        .expect("Parse RFC 7807 ProblemDetails");
    assert_eq!(problem.status, 429);
    assert_eq!(problem.title, "Too Many Requests");
    assert!(problem.detail.contains("Rate limit exceeded"));

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_auth_logout_clears_session_cookie() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");

    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "logout_test_secret_32_bytes_long!".to_string(),
        rate_limit_enabled: false,
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
            .expect("Server error");
    });

    let client = reqwest::Client::new();
    let logout_url = format!("http://{local_addr}/api/v1/auth/logout");

    let res = client
        .post(&logout_url)
        .send()
        .await
        .expect("Send logout request");

    assert_eq!(res.status(), reqwest::StatusCode::NO_CONTENT);

    let set_cookie = res
        .headers()
        .get("set-cookie")
        .expect("Missing Set-Cookie header")
        .to_str()
        .expect("Valid UTF-8");

    assert!(set_cookie.contains("roubit_token="));
    assert!(set_cookie.contains("Max-Age=0"));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));

    let _ = shutdown_tx.send(());
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_query_token_rejected_on_standard_rest_endpoints() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");

    let jwt_secret = "query_token_secret_key_32_bytes_long!".to_string();
    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: jwt_secret.clone(),
        rate_limit_enabled: false,
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
            .expect("Server error");
    });

    let client = reqwest::Client::new();

    // 1. Register a valid user
    let register_url = format!("http://{local_addr}/api/v1/auth/register");
    let register_res = client
        .post(&register_url)
        .json(&serde_json::json!({
            "email": "querytest@roubit.app",
            "password": "valid_password_123"
        }))
        .send()
        .await
        .expect("Register request");
    assert_eq!(register_res.status(), reqwest::StatusCode::CREATED);
    let auth_data: serde_json::Value = register_res.json().await.expect("Parse auth response");
    let token = auth_data["token"].as_str().expect("Token str").to_string();

    // 2. Attempt to authenticate using ?token= on /api/v1/habits (must be REJECTED)
    let habits_url = format!("http://{local_addr}/api/v1/habits?token={token}");
    let query_auth_res = client
        .get(&habits_url)
        .send()
        .await
        .expect("Habits request");
    assert_eq!(
        query_auth_res.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "?token= query parameter authentication must be rejected on standard REST endpoints"
    );

    // 3. Normal Bearer token on /api/v1/habits works
    let bearer_res = client
        .get(format!("http://{local_addr}/api/v1/habits"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("Bearer request");
    assert_eq!(bearer_res.status(), reqwest::StatusCode::OK);

    // 4. SSE endpoint /api/v1/notifications/stream?token= DOES accept query token
    let sse_url = format!("http://{local_addr}/api/v1/notifications/stream?token={token}");
    let sse_res = client.get(&sse_url).send().await.expect("SSE request");
    assert_eq!(sse_res.status(), reqwest::StatusCode::OK);
    let content_type = sse_res
        .headers()
        .get("content-type")
        .expect("Content-Type")
        .to_str()
        .expect("UTF-8");
    assert!(content_type.contains("text/event-stream"));
    drop(sse_res);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}
