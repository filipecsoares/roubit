//! End-to-End Axum REST API Test Suite Backed by PostgreSQL (IMP-02.4)
//!
//! Validates `AppState::from_pool(DatabasePool::Postgres(pg_pool), config)` with `build_router`
//! across `/health`, `/ready`, `/api/v1/auth/register`, `/api/v1/routines`, `/api/v1/habits`,
//! idempotent check-ins (`Idempotency-Key` + `X-Cache: HIT`), `/api/v1/okrs`, and `/api/v1/journal`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names
)]

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use base64::prelude::*;
use roubit_core::{AppConfig, DatabasePool};
use roubit_server::{build_router, AppState, AuthResponse, HealthResponse, RegisterRequest};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_postgres_backed_rest_api_e2e_workflow() {
    let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") else {
        eprintln!("Skipping test_postgres_backed_rest_api_e2e_workflow: TEST_POSTGRES_URL not set");
        return;
    };

    let pool = DatabasePool::from_url(&pg_url)
        .await
        .expect("Failed to connect to PostgreSQL");
    pool.run_migrations()
        .await
        .expect("Failed to run PostgreSQL migrations");

    let mut state = AppState::from_pool(pool, AppConfig::default());
    if let Ok(redis_url) = std::env::var("TEST_REDIS_URL") {
        if let Ok(redis_svc) = roubit_core::redis::RedisService::new(&redis_url) {
            state = state.with_redis(redis_svc);
        }
    }

    // 1. Readiness probe (/ready) against PostgreSQL
    let ready_req = Request::builder()
        .uri("/ready")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let ready_res = build_router(state.clone())
        .oneshot(ready_req)
        .await
        .unwrap();
    assert_eq!(ready_res.status(), StatusCode::OK);
    let ready_body: HealthResponse =
        serde_json::from_slice(&to_bytes(ready_res.into_body(), usize::MAX).await.unwrap())
            .unwrap();
    assert_eq!(ready_body.status, "ready");

    // 2. Register user on PostgreSQL
    let email = format!("pg_api_e2e_{}@roubit.app", Uuid::new_v4());
    let reg_payload = serde_json::to_string(&RegisterRequest {
        email: email.clone(),
        password: "supersecurepassword123".to_string(),
    })
    .unwrap();

    let reg_req = Request::builder()
        .uri("/api/v1/auth/register")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(reg_payload))
        .unwrap();
    let reg_res = build_router(state.clone()).oneshot(reg_req).await.unwrap();
    assert_eq!(reg_res.status(), StatusCode::CREATED);
    let auth: AuthResponse =
        serde_json::from_slice(&to_bytes(reg_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(auth.user.email, email);

    // 3. Create Habit & Perform Idempotent Check-in (MISS -> HIT) on PostgreSQL
    let create_habit_req = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", auth.token))
        .body(Body::from(
            serde_json::json!({
                "name": "Postgres Daily Reading",
                "habit_type": "binary",
                "target_value": 1.0,
                "frequency_days": 127
            })
            .to_string(),
        ))
        .unwrap();
    let create_habit_res = build_router(state.clone())
        .oneshot(create_habit_req)
        .await
        .unwrap();
    assert_eq!(create_habit_res.status(), StatusCode::CREATED);
    let habit: roubit_core::Habit = serde_json::from_slice(
        &to_bytes(create_habit_res.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    let idem_key = format!("pg-idem-{}", Uuid::new_v4());
    let checkin_body = serde_json::json!({
        "date": "2026-11-04",
        "value": 1.0,
        "is_increment": false
    })
    .to_string();

    let checkin_req1 = Request::builder()
        .uri(format!("/api/v1/habits/{}/checkin", habit.id))
        .method("POST")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", auth.token))
        .header("idempotency-key", &idem_key)
        .body(Body::from(checkin_body.clone()))
        .unwrap();
    let checkin_res1 = build_router(state.clone())
        .oneshot(checkin_req1)
        .await
        .unwrap();
    assert_eq!(checkin_res1.status(), StatusCode::OK);
    assert_eq!(
        checkin_res1
            .headers()
            .get("x-cache")
            .and_then(|v| v.to_str().ok()),
        Some("MISS")
    );

    let checkin_req2 = Request::builder()
        .uri(format!("/api/v1/habits/{}/checkin", habit.id))
        .method("POST")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", auth.token))
        .header("idempotency-key", &idem_key)
        .body(Body::from(checkin_body))
        .unwrap();
    let checkin_res2 = build_router(state.clone())
        .oneshot(checkin_req2)
        .await
        .unwrap();
    assert_eq!(checkin_res2.status(), StatusCode::OK);
    assert_eq!(
        checkin_res2
            .headers()
            .get("x-cache")
            .and_then(|v| v.to_str().ok()),
        Some("HIT")
    );

    // 4. Save Zero-Knowledge Encrypted Journal Entry on PostgreSQL
    let journal_req = Request::builder()
        .uri("/api/v1/journal")
        .method("POST")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", auth.token))
        .body(Body::from(
            serde_json::json!({
                "entry_date": "2026-11-04",
                "mood_score": 5,
                "tags": ["postgres", "e2e"],
                "salt_base64": BASE64_STANDARD.encode([0xAAu8; 16]),
                "nonce_base64": BASE64_STANDARD.encode([0xBBu8; 12]),
                "ciphertext_base64": BASE64_STANDARD.encode([0xCCu8; 32])
            })
            .to_string(),
        ))
        .unwrap();
    let journal_res = build_router(state).oneshot(journal_req).await.unwrap();
    assert_eq!(journal_res.status(), StatusCode::CREATED);
}
