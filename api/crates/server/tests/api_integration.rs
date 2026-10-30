//! In-Memory Axum REST API Integration & Security Tests (Tower `ServiceExt`)
//!
//! Executes end-to-end integration flows against an in-memory Axum router without binding
//! real network TCP sockets, validating authentication, idempotency caching, RFC 7807 error
//! contracts, and zero-knowledge journal storage.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use base64::prelude::*;
use roubit_core::{create_sqlite_pool, AppConfig};
use roubit_server::{
    build_router, AppState, AuthResponse, EncryptedJournalResponse, ProblemDetails, RegisterRequest,
};
use tower::ServiceExt;

/// Fixture initializing an in-memory SQLite pool, running migrations, and building the Axum router
async fn setup_test_api_app() -> Router {
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
    build_router(state)
}

/// Helper registering a user and returning the issued JWT bearer token
async fn register_and_get_token(app: &Router, email: &str) -> String {
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
#[allow(clippy::too_many_lines)]
async fn test_auth_and_protected_habit_endpoint() {
    let app = setup_test_api_app().await;

    // 1. Attempt unauthenticated call -> 401 Unauthorized
    let unauth_req = Request::builder()
        .uri("/api/v1/habits")
        .method("GET")
        .body(Body::empty())
        .expect("Build unauth request");

    let unauth_res = app
        .clone()
        .oneshot(unauth_req)
        .await
        .expect("Execute request");
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

    let unauth_bytes = to_bytes(unauth_res.into_body(), usize::MAX)
        .await
        .expect("Read bytes");
    let problem: ProblemDetails =
        serde_json::from_slice(&unauth_bytes).expect("Parse RFC 7807 problem details");
    assert_eq!(problem.status, 401);

    // 2. Register test user and acquire JWT
    let token = register_and_get_token(&app, "tester@roubit.app").await;

    // 3. Make authenticated call with Bearer token -> 200 OK
    let auth_req = Request::builder()
        .uri("/api/v1/habits")
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("Build auth request");

    let auth_res = app
        .clone()
        .oneshot(auth_req)
        .await
        .expect("Execute request");
    assert_eq!(auth_res.status(), StatusCode::OK);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_idempotency_key_duplicate_prevention_in_memory() {
    let app = setup_test_api_app().await;
    let token = register_and_get_token(&app, "idempotent_tester@roubit.app").await;

    let habit_payload = serde_json::json!({
        "name": "Morning Pushups",
        "habit_type": "numeric",
        "target_value": 50.0,
        "unit": "reps"
    });
    let payload_bytes = serde_json::to_vec(&habit_payload).expect("Serialize habit payload");

    // 1. Initial request with Idempotency-Key -> 201 Created & X-Cache: MISS
    let key = "habit-creation-key-42";
    let req1 = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", key)
        .body(Body::from(payload_bytes.clone()))
        .expect("Build request 1");

    let res1 = app.clone().oneshot(req1).await.expect("Execute request 1");
    assert_eq!(res1.status(), StatusCode::CREATED);
    assert_eq!(
        res1.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("MISS")
    );
    let body1 = to_bytes(res1.into_body(), usize::MAX)
        .await
        .expect("Read body 1");

    // 2. Replay identical request with same Idempotency-Key -> 201 Created & X-Cache: HIT
    let req2 = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", key)
        .body(Body::from(payload_bytes))
        .expect("Build request 2");

    let res2 = app.clone().oneshot(req2).await.expect("Execute request 2");
    assert_eq!(res2.status(), StatusCode::CREATED);
    assert_eq!(
        res2.headers().get("x-cache").and_then(|v| v.to_str().ok()),
        Some("HIT")
    );
    let body2 = to_bytes(res2.into_body(), usize::MAX)
        .await
        .expect("Read body 2");
    assert_eq!(body1, body2);

    // 3. Request with same Idempotency-Key but different payload -> 409 Conflict
    let altered_payload = serde_json::json!({
        "name": "Completely Different Habit",
        "habit_type": "binary",
        "target_value": 1.0
    });
    let altered_bytes = serde_json::to_vec(&altered_payload).expect("Serialize altered");

    let req3 = Request::builder()
        .uri("/api/v1/habits")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", key)
        .body(Body::from(altered_bytes))
        .expect("Build request 3");

    let res3 = app.clone().oneshot(req3).await.expect("Execute request 3");
    assert_eq!(res3.status(), StatusCode::CONFLICT);
    let body3 = to_bytes(res3.into_body(), usize::MAX)
        .await
        .expect("Read body 3");
    let conflict_problem: ProblemDetails =
        serde_json::from_slice(&body3).expect("Parse conflict problem details");
    assert_eq!(conflict_problem.status, 409);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_unauthenticated_and_malformed_requests() {
    let app = setup_test_api_app().await;

    // 1. Invalid JWT signature -> 401 Unauthorized
    let bad_jwt_req = Request::builder()
        .uri("/api/v1/auth/me")
        .method("GET")
        .header(
            "authorization",
            "Bearer invalid-signature-token",
        )
        .body(Body::empty())
        .expect("Build bad JWT request");

    let bad_jwt_res = app
        .clone()
        .oneshot(bad_jwt_req)
        .await
        .expect("Execute request");
    assert_eq!(bad_jwt_res.status(), StatusCode::UNAUTHORIZED);

    // 2. Malformed JSON payload to /api/v1/auth/register -> 400 Bad Request
    let malformed_req = Request::builder()
        .uri("/api/v1/auth/register")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("{ \"email\": \"broken-json..."))
        .expect("Build malformed request");

    let malformed_res = app
        .clone()
        .oneshot(malformed_req)
        .await
        .expect("Execute request");
    assert_eq!(malformed_res.status(), StatusCode::BAD_REQUEST);

    // 3. Fallback 404 handler returns RFC 7807 Problem Details
    let not_found_req = Request::builder()
        .uri("/api/v1/does-not-exist")
        .method("GET")
        .body(Body::empty())
        .expect("Build 404 request");

    let not_found_res = app
        .clone()
        .oneshot(not_found_req)
        .await
        .expect("Execute request");
    assert_eq!(not_found_res.status(), StatusCode::NOT_FOUND);
    let nf_bytes = to_bytes(not_found_res.into_body(), usize::MAX)
        .await
        .expect("Read 404 bytes");
    let nf_problem: ProblemDetails =
        serde_json::from_slice(&nf_bytes).expect("Parse 404 problem details");
    assert_eq!(nf_problem.status, 404);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_zero_knowledge_journal_contract_in_memory() {
    let app = setup_test_api_app().await;
    let token = register_and_get_token(&app, "zk_tester@roubit.app").await;

    let fake_salt = BASE64_STANDARD.encode([0x55u8; 16]);
    let fake_nonce = BASE64_STANDARD.encode([0x66u8; 12]);
    let fake_ciphertext = BASE64_STANDARD.encode([0x77u8; 32]);

    // 1. Reject plaintext content field (Zero-Knowledge violation)
    let zk_violation_payload = serde_json::json!({
        "entry_date": "2026-10-30",
        "mood_score": 4,
        "tags": ["reflection"],
        "salt_base64": fake_salt,
        "nonce_base64": fake_nonce,
        "ciphertext_base64": fake_ciphertext,
        "content": "Secret unencrypted text that violates zero-knowledge"
    });
    let req_violation = Request::builder()
        .uri("/api/v1/journal")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&zk_violation_payload).expect("Serialize"),
        ))
        .expect("Build violation request");

    let res_violation = app
        .clone()
        .oneshot(req_violation)
        .await
        .expect("Execute request");
    assert_eq!(res_violation.status(), StatusCode::BAD_REQUEST);

    // 2. Save valid encrypted journal entry
    let valid_payload = serde_json::json!({
        "entry_date": "2026-10-30",
        "mood_score": 4,
        "tags": ["reflection", "growth"],
        "salt_base64": fake_salt,
        "nonce_base64": fake_nonce,
        "ciphertext_base64": fake_ciphertext
    });
    let req_save = Request::builder()
        .uri("/api/v1/journal")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&valid_payload).expect("Serialize"),
        ))
        .expect("Build save request");

    let res_save = app
        .clone()
        .oneshot(req_save)
        .await
        .expect("Execute request");
    assert_eq!(res_save.status(), StatusCode::CREATED);

    let save_bytes = to_bytes(res_save.into_body(), usize::MAX)
        .await
        .expect("Read save bytes");
    let saved_entry: EncryptedJournalResponse =
        serde_json::from_slice(&save_bytes).expect("Parse saved response");
    assert_eq!(saved_entry.salt_base64, fake_salt);
    assert_eq!(saved_entry.nonce_base64, fake_nonce);
    assert_eq!(saved_entry.ciphertext_base64, fake_ciphertext);

    // 3. Fetch entry by ID
    let req_get = Request::builder()
        .uri(format!("/api/v1/journal/{}", saved_entry.id))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("Build get request");

    let res_get = app.clone().oneshot(req_get).await.expect("Execute request");
    assert_eq!(res_get.status(), StatusCode::OK);
}
