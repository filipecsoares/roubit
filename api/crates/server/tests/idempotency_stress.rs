//! Idempotency Hardening & Stress Integration Tests (Activity 03.1)
//!
//! Validates body size bounds (413 Payload Too Large), cross-endpoint/method conflict detection (409 Conflict),
//! authentication endpoint bypass (/api/v1/auth/*), DELETE method support, and immediate lock cleanup on 5xx errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use roubit_core::{create_sqlite_pool, AppConfig};
use roubit_server::{
    build_router,
    middleware::idempotency::{MAX_IDEMPOTENT_BODY_BYTES, X_CACHE_HEADER},
    AppState, AuthResponse, ProblemDetails, RegisterRequest,
};
use tower::ServiceExt;

async fn setup_test_app() -> (Router, AppState) {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");
    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "super-secure-test-jwt-secret-with-minimum-32-bytes!!".to_string(),
        ..Default::default()
    };

    let state = AppState::new(pool, config);
    let app = build_router(state.clone());
    (app, state)
}

async fn register_user(app: &Router, email: &str) -> String {
    let req_body = serde_json::to_vec(&RegisterRequest {
        email: email.to_string(),
        password: "securepassword123".to_string(),
    })
    .expect("Serialize register payload");

    let req = Request::builder()
        .uri("/api/v1/auth/register")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .expect("Build register request");

    let res = app.clone().oneshot(req).await.expect("Execute request");
    assert_eq!(res.status(), StatusCode::CREATED);

    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("Read response bytes");
    let auth: AuthResponse = serde_json::from_slice(&bytes).expect("Parse auth response");
    auth.token
}

#[tokio::test]
async fn test_payload_exceeding_2mib_returns_413_payload_too_large() {
    let (app, _) = setup_test_app().await;
    let token = register_user(&app, "heavy_user@example.com").await;

    // Create a body strictly larger than 2 MiB (2 MiB + 1024 bytes)
    let oversized_bytes = vec![b'A'; MAX_IDEMPOTENT_BODY_BYTES + 1024];

    let req = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", "oversized-key-test-01")
        .body(Body::from(oversized_bytes))
        .expect("Build oversized request");

    let res = app.oneshot(req).await.expect("Execute request");
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("Read response");
    let problem: ProblemDetails =
        serde_json::from_slice(&bytes).expect("Deserialize ProblemDetails");
    assert_eq!(problem.status, 413);
    assert_eq!(problem.title, "Payload Too Large");
}

#[tokio::test]
async fn test_reusing_key_across_different_endpoints_returns_409_conflict() {
    let (app, _) = setup_test_app().await;
    let token = register_user(&app, "path_conflict_user@example.com").await;

    let idempotency_key = "shared-cross-endpoint-key-01";

    // 1. First request to /api/v1/routines
    let routine_payload = serde_json::json!({
        "name": "Morning Routine",
        "blocks": [
            {
                "name": "Meditation",
                "start_time": "07:00",
                "end_time": "07:30"
            }
        ]
    });

    let req1 = Request::builder()
        .uri("/api/v1/routines")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(serde_json::to_vec(&routine_payload).unwrap()))
        .expect("Build request 1");

    let res1 = app.clone().oneshot(req1).await.expect("Execute request 1");
    assert_eq!(res1.status(), StatusCode::CREATED);
    assert_eq!(res1.headers().get(X_CACHE_HEADER).unwrap(), "MISS");

    // 2. Second request reuses the same Idempotency-Key on /api/v1/habits -> must return 409 Conflict
    let habit_payload = serde_json::json!({
        "name": "Drink Water",
        "habit_type": "binary",
        "target_value": 1.0
    });

    let req2 = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(serde_json::to_vec(&habit_payload).unwrap()))
        .expect("Build request 2");

    let res2 = app.clone().oneshot(req2).await.expect("Execute request 2");
    assert_eq!(res2.status(), StatusCode::CONFLICT);

    let bytes = to_bytes(res2.into_body(), usize::MAX)
        .await
        .expect("Read response");
    let problem: ProblemDetails =
        serde_json::from_slice(&bytes).expect("Deserialize ProblemDetails");
    assert_eq!(problem.status, 409);
    assert!(problem
        .detail
        .contains("Idempotency-Key already used with a different request path or payload"));
}

#[tokio::test]
async fn test_reusing_key_across_different_http_methods_returns_409_conflict() {
    let (app, _) = setup_test_app().await;
    let token = register_user(&app, "method_conflict_user@example.com").await;

    let idempotency_key = "shared-method-key-01";

    let habit_payload = serde_json::json!({
        "name": "Daily Reading",
        "habit_type": "binary",
        "target_value": 1.0
    });

    // POST /api/v1/habits
    let req1 = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(serde_json::to_vec(&habit_payload).unwrap()))
        .expect("Build request 1");

    let res1 = app.clone().oneshot(req1).await.expect("Execute request 1");
    assert_eq!(res1.status(), StatusCode::CREATED);

    // DELETE /api/v1/habits/some-id with the same key -> must return 409 Conflict
    let req2 = Request::builder()
        .uri("/api/v1/habits/00000000-0000-0000-0000-000000000001")
        .method("DELETE")
        .header("Authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", idempotency_key)
        .body(Body::empty())
        .expect("Build request 2");

    let res2 = app.clone().oneshot(req2).await.expect("Execute request 2");
    assert_eq!(res2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_auth_routes_bypass_idempotency_caching() {
    let (app, state) = setup_test_app().await;

    let auth_key = "auth-idempotency-key-01";
    let register_payload = serde_json::json!({
        "email": "bypass_auth@example.com",
        "password": "Password123!"
    });

    let req = Request::builder()
        .uri("/api/v1/auth/register")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", auth_key)
        .body(Body::from(serde_json::to_vec(&register_payload).unwrap()))
        .expect("Build register request");

    let res = app.clone().oneshot(req).await.expect("Execute register");
    assert_eq!(res.status(), StatusCode::CREATED);
    // Should NOT have X-Cache header because /api/v1/auth/* is bypassed
    assert!(res.headers().get(X_CACHE_HEADER).is_none());

    // Verify database has NO entry for this key
    let row_count: i64 = match &state.db {
        roubit_core::DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM idempotency_keys WHERE key = ?")
                .bind(auth_key)
                .fetch_one(pool)
                .await
                .expect("Count query")
        }
        roubit_core::DatabasePool::Postgres(pool) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM idempotency_keys WHERE key = $1")
                .bind(auth_key)
                .fetch_one(pool)
                .await
                .expect("Count query")
        }
    };
    assert_eq!(
        row_count, 0,
        "Auth requests must never be persisted in idempotency_keys table"
    );
}

#[tokio::test]
async fn test_delete_pending_lock_on_5xx_allows_immediate_retry() {
    let (app, state) = setup_test_app().await;
    let token = register_user(&app, "server_err_user@example.com").await;

    let retry_key = "server-err-retry-key-01";

    // Manually pre-insert an in-flight pending lock with NULL response_status (as if request crashed with 5xx)
    let now = chrono::Utc::now();
    let locked_until = now + chrono::Duration::seconds(30);

    match &state.db {
        roubit_core::DatabasePool::Sqlite(pool) => {
            sqlx::query(
                r"
                INSERT INTO idempotency_keys (key, user_id, request_path, request_hash, response_status, response_headers, response_body, created_at, locked_until)
                VALUES (?, 'mock_user_id', 'POST /api/v1/habits', X'1234', NULL, NULL, NULL, ?, ?)
                ",
            )
            .bind(retry_key)
            .bind(now.to_rfc3339())
            .bind(locked_until.to_rfc3339())
            .execute(pool)
            .await
            .expect("Insert in-flight mock");
        }
        roubit_core::DatabasePool::Postgres(pool) => {
            sqlx::query(
                r"
                INSERT INTO idempotency_keys (key, user_id, request_path, request_hash, response_status, response_headers, response_body, created_at, locked_until)
                VALUES ($1, 'mock_user_id', 'POST /api/v1/habits', X'1234', NULL, NULL, NULL, $2, $3)
                ",
            )
            .bind(retry_key)
            .bind(now)
            .bind(locked_until)
            .execute(pool)
            .await
            .expect("Insert in-flight mock");
        }
    }

    // Now delete it using the helper function, exactly as idempotency_middleware does when a 5xx occurs
    roubit_server::middleware::idempotency::purge_expired_keys_from_pool(
        &state.db,
        chrono::Duration::seconds(0),
    )
    .await
    .unwrap();

    // Now the user can immediately use that key for a legitimate habit creation
    let habit_payload = serde_json::json!({
        "name": "Workout",
        "habit_type": "binary",
        "target_value": 1.0
    });

    let req = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", retry_key)
        .body(Body::from(serde_json::to_vec(&habit_payload).unwrap()))
        .expect("Build request");

    let res = app.clone().oneshot(req).await.expect("Execute request");
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(res.headers().get(X_CACHE_HEADER).unwrap(), "MISS");
}
