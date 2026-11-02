//! Multi-Tenant Security & IDOR Prevention Integration Test Suite (RF01-RF04, RNF01, RNF03)
//!
//! Validates that two distinct authenticated tenants (Alice and Bob) have strict data isolation
//! across all core domain modules (Habits, Routines, OKRs, and Zero-Knowledge Journaling).
//! Guarantees that cross-tenant read, update, check-in, or deletion attempts fail with HTTP 404
//! to prevent resource existence enumeration (IDOR protection).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::option_if_let_else
)]

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use base64::prelude::*;
use chrono::{NaiveDate, Utc};
use roubit_core::{
    create_sqlite_pool, AppConfig, Habit, JournalMetadata, ObjectiveWithKeyResults, Routine,
};
use roubit_server::{
    build_router, AppState, AuthResponse, CreateHabitRequest, CreateKeyResultRequest,
    CreateObjectiveRequest, CreateRoutineRequest, CreateTimeBlockRequest, EncryptedJournalResponse,
    HabitCheckinRequest, LogBlockStatusRequest, ProblemDetails, RegisterRequest,
    SaveEncryptedJournalRequest,
};
use serde::de::DeserializeOwned;
use tower::ServiceExt;
use uuid::Uuid;

/// Initializes an in-memory SQLite pool, executes migrations, and constructs the Axum router
async fn setup_test_app() -> Router {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to initialize in-memory SQLite database");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply SQLite migrations");

    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "super-secure-multi-tenant-test-jwt-secret-min-32-bytes!!".to_string(),
        ..Default::default()
    };

    let state = AppState::new(pool, config);
    build_router(state)
}

/// Registers a user and returns their issued JWT bearer token
async fn register_user(app: &Router, email: &str) -> String {
    let req_body = serde_json::to_vec(&RegisterRequest {
        email: email.to_string(),
        password: "TenantPassword123!".to_string(),
    })
    .expect("Serialize register payload");

    let req = Request::builder()
        .uri("/api/v1/auth/register")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(req_body))
        .expect("Build register request");

    let res = app
        .clone()
        .oneshot(req)
        .await
        .expect("Execute register request");
    assert_eq!(res.status(), StatusCode::CREATED);

    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("Read response bytes");
    let auth: AuthResponse = serde_json::from_slice(&bytes).expect("Parse AuthResponse");
    auth.token
}

/// Helper executing an authenticated JSON request and deserializing response body
async fn json_request<T: DeserializeOwned>(
    app: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, Option<T>) {
    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("authorization", format!("Bearer {token}"));

    let body_bytes = if let Some(json_val) = body {
        builder = builder.header("content-type", "application/json");
        serde_json::to_vec(&json_val).expect("Serialize request JSON")
    } else {
        Vec::new()
    };

    let req = builder.body(Body::from(body_bytes)).expect("Build request");

    let res = app.clone().oneshot(req).await.expect("Execute request");
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("Read response bytes");

    if bytes.is_empty() {
        (status, None)
    } else {
        let parsed = serde_json::from_slice::<T>(&bytes).ok();
        (status, parsed)
    }
}

// ============================================================================
// 1. HABITS MULTI-TENANT ISOLATION & IDOR PREVENTION
// ============================================================================

#[tokio::test]
async fn test_habits_multi_tenant_isolation_and_idor() {
    let app = setup_test_app().await;

    let alice_token = register_user(&app, "alice_habits@roubit.app").await;
    let bob_token = register_user(&app, "bob_habits@roubit.app").await;

    // 1. Alice creates habit "Meditation"
    let (status, alice_habit) = json_request::<Habit>(
        &app,
        "POST",
        "/api/v1/habits",
        &alice_token,
        Some(
            serde_json::to_value(CreateHabitRequest {
                name: "Meditation".to_string(),
                description: Some("Daily mindfulness".to_string()),
                habit_type: "binary".to_string(),
                target_value: 1.0,
                unit: None,
                frequency_days: Some(0b0111_1111),
                color_hex: Some("#10B981".to_string()),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let alice_habit = alice_habit.expect("Alice habit returned");
    let alice_habit_id = alice_habit.id;

    // 2. Bob calls GET /api/v1/habits -> must receive empty list (cannot see Alice's habit)
    let (status, bob_habits) =
        json_request::<Vec<serde_json::Value>>(&app, "GET", "/api/v1/habits", &bob_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let bob_habits = bob_habits.expect("Bob habits array");
    assert!(
        bob_habits.is_empty(),
        "Bob should see 0 habits, found {}",
        bob_habits.len()
    );

    // 3. Bob attempts IDOR check-in on Alice's habit -> 404 Not Found
    let today = Utc::now().date_naive();
    let (status, problem) = json_request::<ProblemDetails>(
        &app,
        "POST",
        &format!("/api/v1/habits/{alice_habit_id}/checkin"),
        &bob_token,
        Some(
            serde_json::to_value(HabitCheckinRequest {
                date: today,
                value: 1.0,
                is_increment: Some(false),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob checking in to Alice's habit must return 404"
    );
    assert_eq!(problem.map(|p| p.status), Some(404));

    // 4. Bob attempts IDOR delete on Alice's habit -> 404 Not Found
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/habits/{alice_habit_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob deleting Alice's habit must return 404"
    );

    // 5. Verify Alice's habit is still active and untouched
    let (status, alice_habits) =
        json_request::<Vec<serde_json::Value>>(&app, "GET", "/api/v1/habits", &alice_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let alice_habits = alice_habits.expect("Alice habits array");
    assert_eq!(alice_habits.len(), 1, "Alice should still have 1 habit");

    // 6. Alice can check in to her own habit -> 200 OK
    let (status, checkin) = json_request::<serde_json::Value>(
        &app,
        "POST",
        &format!("/api/v1/habits/{alice_habit_id}/checkin"),
        &alice_token,
        Some(
            serde_json::to_value(HabitCheckinRequest {
                date: today,
                value: 1.0,
                is_increment: Some(false),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(checkin.is_some());

    // 7. Alice deletes her own habit -> 200 OK
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/habits/{alice_habit_id}"),
        &alice_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// 2. ROUTINES & TIME BLOCKS MULTI-TENANT ISOLATION & IDOR PREVENTION
// ============================================================================

#[tokio::test]
async fn test_routines_multi_tenant_isolation_and_idor() {
    let app = setup_test_app().await;

    let alice_token = register_user(&app, "alice_routines@roubit.app").await;
    let bob_token = register_user(&app, "bob_routines@roubit.app").await;

    // 1. Alice creates active routine "Morning Focus" with a 45-min time block
    let (status, alice_routine) = json_request::<Routine>(
        &app,
        "POST",
        "/api/v1/routines",
        &alice_token,
        Some(
            serde_json::to_value(CreateRoutineRequest {
                name: "Morning Focus".to_string(),
                description: Some("Deep work schedule".to_string()),
                is_active: Some(true),
                blocks: vec![CreateTimeBlockRequest {
                    name: "Deep Work Block".to_string(),
                    start_time: "08:00:00".to_string(),
                    end_time: "08:45:00".to_string(),
                    category: Some("focus".to_string()),
                    color_hex: Some("#3B82F6".to_string()),
                    order_index: Some(0),
                }],
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let alice_routine = alice_routine.expect("Alice routine returned");
    let alice_routine_id = alice_routine.id;

    // Retrieve Alice's routine with blocks to obtain block_id
    let (status, alice_active) =
        json_request::<serde_json::Value>(&app, "GET", "/api/v1/routines", &alice_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let alice_active = alice_active.expect("Alice active routine");
    let alice_block_id_str = alice_active["blocks"][0]["id"]
        .as_str()
        .expect("Alice time block ID string");
    let alice_block_id = Uuid::parse_str(alice_block_id_str).expect("Valid block UUID");

    // 2. Bob calls GET /api/v1/routines/all -> receives empty list
    let (status, bob_all) =
        json_request::<Vec<Routine>>(&app, "GET", "/api/v1/routines/all", &bob_token, None).await;
    assert_eq!(status, StatusCode::OK);
    let bob_all = bob_all.expect("Bob routines list");
    assert!(bob_all.is_empty(), "Bob should see 0 routines");

    // 3. Bob calls GET /api/v1/routines -> returns 200 OK with routine: None and blocks: [] (Alice's active routine not leaked)
    let (status, bob_active) =
        json_request::<serde_json::Value>(&app, "GET", "/api/v1/routines", &bob_token, None).await;
    assert_eq!(status, StatusCode::OK);
    let bob_active = bob_active.expect("Bob active routine response");
    assert!(
        bob_active["routine"].is_null(),
        "Bob should have no active routine"
    );
    assert_eq!(bob_active["blocks"].as_array().map(Vec::len), Some(0));

    // 4. Bob attempts IDOR completion on Alice's time block -> 404 Not Found
    let (status, problem) = json_request::<ProblemDetails>(
        &app,
        "POST",
        &format!("/api/v1/routines/blocks/{alice_block_id}/complete"),
        &bob_token,
        Some(
            serde_json::to_value(LogBlockStatusRequest {
                date: Some(Utc::now().date_naive()),
                status: Some("completed".to_string()),
                notes: Some("Attempted cross-tenant tamper".to_string()),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob completing Alice's block must return 404"
    );
    assert_eq!(problem.map(|p| p.status), Some(404));

    // 5. Bob attempts IDOR delete on Alice's routine -> 404 Not Found
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/routines/{alice_routine_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob deleting Alice's routine must return 404"
    );

    // 6. Alice successfully completes her block -> 200 OK
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "POST",
        &format!("/api/v1/routines/blocks/{alice_block_id}/complete"),
        &alice_token,
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 7. Bob creates his own routine "Bob Routine" -> succeeds
    let (status, bob_routine) = json_request::<Routine>(
        &app,
        "POST",
        "/api/v1/routines",
        &bob_token,
        Some(
            serde_json::to_value(CreateRoutineRequest {
                name: "Bob Routine".to_string(),
                description: None,
                is_active: Some(true),
                blocks: vec![CreateTimeBlockRequest {
                    name: "Bob Work".to_string(),
                    start_time: "09:00:00".to_string(),
                    end_time: "10:00:00".to_string(),
                    category: Some("focus".to_string()),
                    color_hex: None,
                    order_index: Some(0),
                }],
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(bob_routine.unwrap().name, "Bob Routine");

    // 8. Bob sees exactly 1 routine in list_routines
    let (status, bob_all) =
        json_request::<Vec<Routine>>(&app, "GET", "/api/v1/routines/all", &bob_token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bob_all.unwrap().len(), 1);
}

// ============================================================================
// 3. OKRS & KEY RESULTS MULTI-TENANT ISOLATION & IDOR PREVENTION
// ============================================================================

#[tokio::test]
async fn test_okrs_multi_tenant_isolation_and_idor() {
    let app = setup_test_app().await;

    let alice_token = register_user(&app, "alice_okrs@roubit.app").await;
    let bob_token = register_user(&app, "bob_okrs@roubit.app").await;

    // 1. Alice creates habit "Alice Habit"
    let (status, alice_habit) = json_request::<Habit>(
        &app,
        "POST",
        "/api/v1/habits",
        &alice_token,
        Some(
            serde_json::to_value(CreateHabitRequest {
                name: "Alice Habit".to_string(),
                description: None,
                habit_type: "numeric".to_string(),
                target_value: 10.0,
                unit: Some("pages".to_string()),
                frequency_days: Some(0b0111_1111),
                color_hex: None,
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let alice_habit_id = alice_habit.unwrap().id;

    // 2. Alice creates Objective "Alice Q4 Objective" with nested Key Result
    let (status, alice_objective) = json_request::<ObjectiveWithKeyResults>(
        &app,
        "POST",
        "/api/v1/okrs",
        &alice_token,
        Some(
            serde_json::to_value(CreateObjectiveRequest {
                title: "Alice Q4 Objective".to_string(),
                description: Some("Scale platform".to_string()),
                quarter: Some("2026-Q4".to_string()),
                start_date: None,
                end_date: None,
                key_results: Some(vec![CreateKeyResultRequest {
                    title: "Read 100 pages".to_string(),
                    initial_value: Some(0.0),
                    target_value: 100.0,
                    unit: Some("pages".to_string()),
                    weight: Some(1.0),
                    linked_habit_id: Some(alice_habit_id),
                }]),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let alice_objective = alice_objective.expect("Alice objective returned");
    let alice_objective_id = alice_objective.objective.id;

    // 3. Bob calls GET /api/v1/okrs -> must return empty list (cannot see Alice's OKRs)
    let (status, bob_okrs) =
        json_request::<Vec<ObjectiveWithKeyResults>>(&app, "GET", "/api/v1/okrs", &bob_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let bob_okrs = bob_okrs.expect("Bob OKRs list");
    assert!(bob_okrs.is_empty(), "Bob should see 0 OKRs");

    // 4. Bob attempts IDOR add Key Result to Alice's objective -> 404 Not Found
    let (status, problem) = json_request::<ProblemDetails>(
        &app,
        "POST",
        &format!("/api/v1/okrs/{alice_objective_id}/key-results"),
        &bob_token,
        Some(
            serde_json::to_value(CreateKeyResultRequest {
                title: "Bob Injected KR".to_string(),
                initial_value: Some(0.0),
                target_value: 50.0,
                unit: Some("%".to_string()),
                weight: Some(1.0),
                linked_habit_id: None,
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob adding KR to Alice's objective must return 404"
    );
    assert_eq!(problem.map(|p| p.status), Some(404));

    // 5. Bob attempts IDOR delete on Alice's objective -> 404 Not Found
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/okrs/{alice_objective_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob deleting Alice's objective must return 404"
    );

    // 6. Bob creates his own Objective "Bob Objective"
    let (status, bob_objective) = json_request::<ObjectiveWithKeyResults>(
        &app,
        "POST",
        "/api/v1/okrs",
        &bob_token,
        Some(
            serde_json::to_value(CreateObjectiveRequest {
                title: "Bob Objective".to_string(),
                description: None,
                quarter: Some("2026-Q4".to_string()),
                start_date: None,
                end_date: None,
                key_results: None,
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let bob_objective_id = bob_objective.unwrap().objective.id;

    // 7. Bob attempts to link Alice's habit to his own Key Result -> 404 Not Found (Cross-tenant habit link rejected)
    let (status, _) = json_request::<ProblemDetails>(
        &app,
        "POST",
        &format!("/api/v1/okrs/{bob_objective_id}/key-results"),
        &bob_token,
        Some(
            serde_json::to_value(CreateKeyResultRequest {
                title: "Tampered Habit Link KR".to_string(),
                initial_value: Some(0.0),
                target_value: 10.0,
                unit: Some("pts".to_string()),
                weight: Some(1.0),
                linked_habit_id: Some(alice_habit_id),
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Linking Alice's habit to Bob's KR must be rejected with 404"
    );

    // 8. Alice's objective remains intact with its original KR
    let (status, alice_okrs) = json_request::<Vec<ObjectiveWithKeyResults>>(
        &app,
        "GET",
        "/api/v1/okrs",
        &alice_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let alice_okrs = alice_okrs.unwrap();
    assert_eq!(alice_okrs.len(), 1);
    assert_eq!(alice_okrs[0].key_results.len(), 1);
}

// ============================================================================
// 4. ZERO-KNOWLEDGE JOURNAL MULTI-TENANT ISOLATION & IDOR PREVENTION
// ============================================================================

#[tokio::test]
async fn test_journal_multi_tenant_isolation_and_idor() {
    let app = setup_test_app().await;

    let alice_token = register_user(&app, "alice_journal@roubit.app").await;
    let bob_token = register_user(&app, "bob_journal@roubit.app").await;

    // 1. Alice creates an encrypted journal entry for today
    let date = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let salt = [1u8; 16];
    let nonce = [2u8; 12];
    let ciphertext = vec![3u8; 32];

    let (status, alice_entry) = json_request::<EncryptedJournalResponse>(
        &app,
        "POST",
        "/api/v1/journal",
        &alice_token,
        Some(
            serde_json::to_value(SaveEncryptedJournalRequest {
                id: None,
                entry_date: date,
                mood_score: Some(5),
                tags: Some(vec!["confidential".to_string(), "alice".to_string()]),
                salt_base64: BASE64_STANDARD.encode(salt),
                nonce_base64: BASE64_STANDARD.encode(nonce),
                ciphertext_base64: BASE64_STANDARD.encode(&ciphertext),
                content: None,
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let alice_entry = alice_entry.expect("Alice journal entry returned");
    let alice_entry_id = alice_entry.id;

    // 2. Bob calls GET /api/v1/journal/metadata -> receives empty list
    let (status, bob_meta) = json_request::<Vec<JournalMetadata>>(
        &app,
        "GET",
        "/api/v1/journal/metadata",
        &bob_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let bob_meta = bob_meta.expect("Bob metadata list");
    assert!(
        bob_meta.is_empty(),
        "Bob should see 0 journal entries, found {}",
        bob_meta.len()
    );

    // 3. Bob attempts IDOR fetch on Alice's encrypted journal entry -> 404 Not Found
    let (status, problem) = json_request::<ProblemDetails>(
        &app,
        "GET",
        &format!("/api/v1/journal/{alice_entry_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob fetching Alice's entry must return 404"
    );
    assert_eq!(problem.map(|p| p.status), Some(404));

    // 4. Bob attempts IDOR delete on Alice's encrypted journal entry -> 404 Not Found
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/journal/{alice_entry_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob deleting Alice's entry must return 404"
    );

    // 5. Bob creates his own encrypted journal entry for the same date
    let bob_salt = [4u8; 16];
    let bob_nonce = [5u8; 12];
    let bob_ciphertext = vec![6u8; 32];
    let (status, bob_entry) = json_request::<EncryptedJournalResponse>(
        &app,
        "POST",
        "/api/v1/journal",
        &bob_token,
        Some(
            serde_json::to_value(SaveEncryptedJournalRequest {
                id: None,
                entry_date: date,
                mood_score: Some(3),
                tags: Some(vec!["bob-notes".to_string()]),
                salt_base64: BASE64_STANDARD.encode(bob_salt),
                nonce_base64: BASE64_STANDARD.encode(bob_nonce),
                ciphertext_base64: BASE64_STANDARD.encode(&bob_ciphertext),
                content: None,
            })
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let bob_entry_id = bob_entry.unwrap().id;

    // 6. Alice sees only her metadata, Bob sees only his metadata
    let (status, alice_meta) = json_request::<Vec<JournalMetadata>>(
        &app,
        "GET",
        "/api/v1/journal/metadata",
        &alice_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let alice_meta = alice_meta.unwrap();
    assert_eq!(alice_meta.len(), 1);
    assert_eq!(alice_meta[0].id, alice_entry_id);

    let (status, bob_meta) = json_request::<Vec<JournalMetadata>>(
        &app,
        "GET",
        "/api/v1/journal/metadata",
        &bob_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let bob_meta = bob_meta.unwrap();
    assert_eq!(bob_meta.len(), 1);
    assert_eq!(bob_meta[0].id, bob_entry_id);

    // 7. Alice deletes her entry -> 200 OK. Bob's entry remains untouched.
    let (status, _) = json_request::<serde_json::Value>(
        &app,
        "DELETE",
        &format!("/api/v1/journal/{alice_entry_id}"),
        &alice_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, bob_entry_after) = json_request::<EncryptedJournalResponse>(
        &app,
        "GET",
        &format!("/api/v1/journal/{bob_entry_id}"),
        &bob_token,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(bob_entry_after.is_some());
}

// ============================================================================
// 5. CONCURRENT CROSS-TENANT ISOLATION UNDER INTERLEAVED EXECUTION
// ============================================================================

#[tokio::test]
async fn test_multi_tenant_concurrent_isolation() {
    let app = setup_test_app().await;

    let alice_token = register_user(&app, "alice_concurrent@roubit.app").await;
    let bob_token = register_user(&app, "bob_concurrent@roubit.app").await;

    // Concurrently create 5 habits for Alice and 5 habits for Bob
    let mut handles = Vec::new();

    for i in 0..5 {
        let app_clone = app.clone();
        let token = alice_token.clone();
        handles.push(tokio::spawn(async move {
            let (status, habit) = json_request::<Habit>(
                &app_clone,
                "POST",
                "/api/v1/habits",
                &token,
                Some(
                    serde_json::to_value(CreateHabitRequest {
                        name: format!("Alice Habit {i}"),
                        description: None,
                        habit_type: "binary".to_string(),
                        target_value: 1.0,
                        unit: None,
                        frequency_days: Some(0b0111_1111),
                        color_hex: None,
                    })
                    .unwrap(),
                ),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
            habit.unwrap()
        }));

        let app_clone = app.clone();
        let token = bob_token.clone();
        handles.push(tokio::spawn(async move {
            let (status, habit) = json_request::<Habit>(
                &app_clone,
                "POST",
                "/api/v1/habits",
                &token,
                Some(
                    serde_json::to_value(CreateHabitRequest {
                        name: format!("Bob Habit {i}"),
                        description: None,
                        habit_type: "numeric".to_string(),
                        target_value: 10.0,
                        unit: Some("mins".to_string()),
                        frequency_days: Some(0b0111_1111),
                        color_hex: None,
                    })
                    .unwrap(),
                ),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
            habit.unwrap()
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Verify Alice sees exactly her 5 habits
    let (status, alice_habits) =
        json_request::<Vec<serde_json::Value>>(&app, "GET", "/api/v1/habits", &alice_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let alice_list = alice_habits.unwrap();
    assert_eq!(alice_list.len(), 5, "Alice must see exactly 5 habits");
    for item in &alice_list {
        assert!(item["habit"]["name"]
            .as_str()
            .unwrap()
            .starts_with("Alice Habit"));
    }

    // Verify Bob sees exactly his 5 habits
    let (status, bob_habits) =
        json_request::<Vec<serde_json::Value>>(&app, "GET", "/api/v1/habits", &bob_token, None)
            .await;
    assert_eq!(status, StatusCode::OK);
    let bob_list = bob_habits.unwrap();
    assert_eq!(bob_list.len(), 5, "Bob must see exactly 5 habits");
    for item in &bob_list {
        assert!(item["habit"]["name"]
            .as_str()
            .unwrap()
            .starts_with("Bob Habit"));
    }
}
