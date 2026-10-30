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

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_tcp_domain_controllers_routines_habits_and_okrs() {
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

    // 1. Register user
    let reg_url = format!("http://{local_addr}/api/v1/auth/register");
    let reg_res = client
        .post(&reg_url)
        .json(&roubit_server::RegisterRequest {
            email: "domain_user@roubit.app".to_string(),
            password: "supersecurepassword123".to_string(),
        })
        .send()
        .await
        .expect("Register failed");
    let auth: roubit_server::AuthResponse = reg_res.json().await.expect("Valid auth");

    // 2. Routines CRUD
    let routines_url = format!("http://{local_addr}/api/v1/routines");
    let create_routine_res = client
        .post(&routines_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "name": "Morning Productivity",
            "description": "Deep work routine",
            "is_active": true,
            "blocks": [
                {
                    "name": "Meditation",
                    "start_time": "08:00:00",
                    "end_time": "08:30:00",
                    "category": "Mindfulness",
                    "color_hex": "#10B981"
                },
                {
                    "name": "Deep Coding",
                    "start_time": "08:30:00",
                    "end_time": "10:30:00",
                    "category": "Deep Work",
                    "color_hex": "#6366F1"
                }
            ]
        }))
        .send()
        .await
        .expect("Create routine failed");
    assert_eq!(create_routine_res.status(), reqwest::StatusCode::CREATED);
    let created_routine: roubit_core::Routine =
        create_routine_res.json().await.expect("Parse routine");
    assert_eq!(created_routine.name, "Morning Productivity");

    // Fetch active routine
    let get_routine_res = client
        .get(&routines_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("Get routine failed");
    assert_eq!(get_routine_res.status(), reqwest::StatusCode::OK);
    let routine_prog: roubit_server::controllers::RoutineWithProgressResponse = get_routine_res
        .json()
        .await
        .expect("Parse routine with progress");
    assert_eq!(routine_prog.blocks.len(), 2);
    let block1_id = routine_prog.blocks[0].id;

    // Complete time block
    let complete_url = format!("http://{local_addr}/api/v1/routines/blocks/{block1_id}/complete");
    let complete_res = client
        .post(&complete_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "status": "completed",
            "notes": "Done effortlessly"
        }))
        .send()
        .await
        .expect("Complete block failed");
    assert_eq!(complete_res.status(), reqwest::StatusCode::OK);

    // 3. Habits CRUD & Idempotent check-in
    let habits_url = format!("http://{local_addr}/api/v1/habits");
    let create_habit_res = client
        .post(&habits_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "name": "Drink Water",
            "habit_type": "numeric",
            "target_value": 2.5,
            "unit": "Liters",
            "frequency_days": 127
        }))
        .send()
        .await
        .expect("Create habit failed");
    assert_eq!(create_habit_res.status(), reqwest::StatusCode::CREATED);
    let habit: roubit_core::Habit = create_habit_res.json().await.expect("Parse habit");

    // Checkin with Idempotency-Key
    let checkin_url = format!("http://{local_addr}/api/v1/habits/{}/checkin", habit.id);
    let checkin_key = "habit-checkin-uuid-key-1";
    let checkin_res1 = client
        .post(&checkin_url)
        .bearer_auth(&auth.token)
        .header("idempotency-key", checkin_key)
        .json(&serde_json::json!({
            "date": "2026-10-30",
            "value": 1.5,
            "is_increment": false
        }))
        .send()
        .await
        .expect("Checkin 1 failed");
    assert_eq!(checkin_res1.status(), reqwest::StatusCode::OK);
    assert_eq!(
        checkin_res1
            .headers()
            .get("x-cache")
            .and_then(|v| v.to_str().ok()),
        Some("MISS")
    );

    // Replay checkin with same Idempotency-Key
    let checkin_res2 = client
        .post(&checkin_url)
        .bearer_auth(&auth.token)
        .header("idempotency-key", checkin_key)
        .json(&serde_json::json!({
            "date": "2026-10-30",
            "value": 1.5,
            "is_increment": false
        }))
        .send()
        .await
        .expect("Checkin 2 failed");
    assert_eq!(checkin_res2.status(), reqwest::StatusCode::OK);
    assert_eq!(
        checkin_res2
            .headers()
            .get("x-cache")
            .and_then(|v| v.to_str().ok()),
        Some("HIT")
    );

    // List habits
    let list_habits_res = client
        .get(&habits_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("List habits failed");
    assert_eq!(list_habits_res.status(), reqwest::StatusCode::OK);

    // 4. OKRs CRUD
    let okrs_url = format!("http://{local_addr}/api/v1/okrs");
    let create_okr_res = client
        .post(&okrs_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "title": "Master Rust Web Architecture",
            "description": "High performance APIs with Axum",
            "quarter": "Q4-2026",
            "key_results": [
                {
                    "title": "Drink water for cognitive stamina",
                    "initial_value": 0.0,
                    "target_value": 100.0,
                    "unit": "%",
                    "linked_habit_id": habit.id
                }
            ]
        }))
        .send()
        .await
        .expect("Create OKR failed");
    assert_eq!(create_okr_res.status(), reqwest::StatusCode::CREATED);

    // List OKRs
    let list_okrs_res = client
        .get(&okrs_url)
        .query(&[("quarter", "Q4-2026")])
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("List OKRs failed");
    assert_eq!(list_okrs_res.status(), reqwest::StatusCode::OK);
    let okrs: Vec<roubit_core::ObjectiveWithKeyResults> =
        list_okrs_res.json().await.expect("Parse OKRs");
    assert_eq!(okrs.len(), 1);
    assert_eq!(okrs[0].key_results.len(), 1);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_tcp_zero_knowledge_journal_endpoints() {
    use base64::prelude::*;

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

    // 1. Register user
    let reg_url = format!("http://{local_addr}/api/v1/auth/register");
    let reg_res = client
        .post(&reg_url)
        .json(&roubit_server::RegisterRequest {
            email: "journal_user@roubit.app".to_string(),
            password: "supersecurepassword123".to_string(),
        })
        .send()
        .await
        .expect("Register failed");
    let auth: roubit_server::AuthResponse = reg_res.json().await.expect("Valid auth");

    // 2. Reject zero-knowledge violation (plaintext content)
    let journal_url = format!("http://{local_addr}/api/v1/journal");
    let fake_salt = BASE64_STANDARD.encode([0x42u8; 16]);
    let fake_nonce = BASE64_STANDARD.encode([0x24u8; 12]);
    let fake_ciphertext = BASE64_STANDARD.encode([0xAAu8; 32]);

    let violation_res = client
        .post(&journal_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "entry_date": "2026-10-30",
            "mood_score": 5,
            "tags": ["focus"],
            "salt_base64": fake_salt,
            "nonce_base64": fake_nonce,
            "ciphertext_base64": fake_ciphertext,
            "content": "Plaintext secret reflection that must be rejected"
        }))
        .send()
        .await
        .expect("Violation request failed");
    assert_eq!(violation_res.status(), reqwest::StatusCode::BAD_REQUEST);

    // 3. Reject invalid salt length (< 16 bytes)
    let invalid_salt_res = client
        .post(&journal_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "entry_date": "2026-10-30",
            "mood_score": 5,
            "tags": ["focus"],
            "salt_base64": BASE64_STANDARD.encode([1u8; 8]),
            "nonce_base64": fake_nonce,
            "ciphertext_base64": fake_ciphertext
        }))
        .send()
        .await
        .expect("Invalid salt request failed");
    assert_eq!(invalid_salt_res.status(), reqwest::StatusCode::BAD_REQUEST);

    // 4. Save valid encrypted journal entry
    let save_res = client
        .post(&journal_url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "entry_date": "2026-10-30",
            "mood_score": 5,
            "tags": ["focus", "deep-work"],
            "salt_base64": fake_salt,
            "nonce_base64": fake_nonce,
            "ciphertext_base64": fake_ciphertext
        }))
        .send()
        .await
        .expect("Save entry failed");
    assert_eq!(save_res.status(), reqwest::StatusCode::CREATED);
    let saved_entry: roubit_server::controllers::EncryptedJournalResponse =
        save_res.json().await.expect("Parse encrypted entry");
    assert_eq!(saved_entry.entry_date.to_string(), "2026-10-30");
    assert_eq!(saved_entry.mood_score, Some(5));
    assert_eq!(saved_entry.tags, vec!["focus", "deep-work"]);
    assert_eq!(saved_entry.salt_base64, fake_salt);
    assert_eq!(saved_entry.nonce_base64, fake_nonce);
    assert_eq!(saved_entry.ciphertext_base64, fake_ciphertext);

    // 5. Query metadata (omits ciphertext payload)
    let metadata_url = format!("http://{local_addr}/api/v1/journal/metadata");
    let meta_res = client
        .get(&metadata_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("Metadata request failed");
    assert_eq!(meta_res.status(), reqwest::StatusCode::OK);
    let meta_list: Vec<roubit_core::JournalMetadata> =
        meta_res.json().await.expect("Parse metadata list");
    assert_eq!(meta_list.len(), 1);
    assert_eq!(meta_list[0].id, saved_entry.id);
    assert_eq!(meta_list[0].ciphertext_bytes_len, 32);
    assert_eq!(meta_list[0].mood_score, Some(5));

    // 6. Fetch complete entry by ID
    let get_entry_url = format!("http://{local_addr}/api/v1/journal/{}", saved_entry.id);
    let get_res = client
        .get(&get_entry_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("Get entry failed");
    assert_eq!(get_res.status(), reqwest::StatusCode::OK);
    let fetched_entry: roubit_server::controllers::EncryptedJournalResponse =
        get_res.json().await.expect("Parse fetched entry");
    assert_eq!(fetched_entry.id, saved_entry.id);
    assert_eq!(fetched_entry.ciphertext_base64, fake_ciphertext);

    // 7. Delete entry
    let del_res = client
        .delete(&get_entry_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("Delete entry failed");
    assert_eq!(del_res.status(), reqwest::StatusCode::OK);

    // Fetch again -> 404
    let not_found_res = client
        .get(&get_entry_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("Get deleted entry failed");
    assert_eq!(not_found_res.status(), reqwest::StatusCode::NOT_FOUND);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}

#[tokio::test]
async fn test_tcp_sse_notification_stream() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");
    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "test_secret_32_characters_long_min!".to_string(),
        ..Default::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral port");
    let local_addr = listener.local_addr().expect("Failed to get local address");

    let state = AppState::new(pool.clone(), config);
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
    let reg_body = serde_json::json!({
        "email": "sse_user@roubit.app",
        "password": "Password123!"
    });
    let reg_res = client
        .post(&reg_url)
        .json(&reg_body)
        .send()
        .await
        .expect("Register failed");
    assert_eq!(reg_res.status(), reqwest::StatusCode::CREATED);
    let auth: roubit_server::AuthResponse = reg_res.json().await.expect("Valid auth response");

    // Test notification hydration
    let hydrated =
        roubit_server::hydrate_reminder(&pool, &auth.user.id.to_string(), "habit", "non_existent")
            .await;
    assert_eq!(hydrated.entity_type, "habit");
    assert!(hydrated.title.contains("Hábito"));

    // Connect to SSE stream
    let sse_url = format!("http://{local_addr}/api/v1/notifications/stream");
    let sse_res = client
        .get(&sse_url)
        .bearer_auth(&auth.token)
        .send()
        .await
        .expect("SSE request failed");

    assert_eq!(sse_res.status(), reqwest::StatusCode::OK);
    let content_type = sse_res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .expect("Content-Type header")
        .to_str()
        .expect("Valid utf8");
    assert!(content_type.contains("text/event-stream"));

    drop(sse_res);

    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}
