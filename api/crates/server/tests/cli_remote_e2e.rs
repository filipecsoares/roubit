#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use assert_cmd::Command;
use roubit_core::{create_sqlite_pool, AppConfig};
use roubit_server::{build_router, AppState};
use uuid::Uuid;

struct EphemeralServer {
    pub base_url: String,
    _shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

async fn start_test_server() -> EphemeralServer {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create memory db");
    roubit_core::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations");

    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "super-secret-jwt-key-for-testing-12345".to_string(),
        ..Default::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral port");
    let local_addr = listener.local_addr().expect("Failed to get local address");

    let state = AppState::new(pool, config);
    let app = build_router(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("Server exited with error");
    });

    let base_url = format!("http://{local_addr}");

    // Wait until server responds to health endpoint
    let client = reqwest::Client::new();
    let health_url = format!("{base_url}/health");
    for _ in 0..50 {
        if let Ok(res) = client.get(&health_url).send().await {
            if res.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    EphemeralServer {
        base_url,
        _shutdown_tx: shutdown_tx,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_remote_end_to_end_workflow() {
    let server = start_test_server().await;
    let client = reqwest::Client::new();

    // 0. Register test user via API endpoint
    let register_url = format!("{}/api/v1/auth/register", server.base_url);
    let reg_res = client
        .post(&register_url)
        .json(&serde_json::json!({
            "email": "testuser@example.com",
            "password": "Password123!"
        }))
        .send()
        .await
        .expect("Register request failed");
    assert_eq!(
        reg_res.status(),
        reqwest::StatusCode::CREATED,
        "Registration must succeed"
    );

    // Setup isolated credentials file path for this test
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let creds_path = temp_dir.path().join("credentials.json");
    let creds_path_str = creds_path.to_str().expect("Valid UTF-8 path");

    // Scenario 1: roubit auth login
    let mut cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    let assert = cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args([
            "auth",
            "login",
            "--url",
            &server.base_url,
            "--email",
            "testuser@example.com",
            "--password",
            "Password123!",
        ])
        .assert();

    assert.success();

    // Verify credentials file was saved and has 0600 permissions
    assert!(
        creds_path.exists(),
        "Credentials file must exist after login"
    );
    #[cfg(unix)]
    {
        let meta = fs::metadata(&creds_path).expect("Read creds metadata");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Credentials file must have 0600 permissions");
    }

    // Verify auth status via CLI
    let mut status_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    status_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args(["auth", "status", "--json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("testuser@example.com"));

    // Scenario 2: Create habit via remote CLI
    let mut create_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    let create_out = create_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args([
            "--remote",
            &server.base_url,
            "habit",
            "create",
            "--name",
            "Morning Workout",
            "--kind",
            "numeric",
            "--target",
            "30",
            "--unit",
            "minutes",
            "--json",
        ])
        .assert()
        .success();

    let create_stdout = String::from_utf8(create_out.get_output().stdout.clone())
        .expect("UTF-8 stdout from create");
    let habit_json: serde_json::Value =
        serde_json::from_str(&create_stdout).expect("Valid JSON response from habit create");
    let habit_id_str = habit_json["id"]
        .as_str()
        .expect("Habit ID in JSON response");
    let habit_uuid = Uuid::parse_str(habit_id_str).expect("Valid Habit UUID");

    // Scenario 3: List habits via remote CLI
    let mut list_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    list_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args(["--remote", &server.base_url, "habit", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Morning Workout"))
        .stdout(predicates::str::contains("30.0 minutes"));

    // Scenario 4: Checkin habit via remote CLI
    let mut checkin_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    checkin_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args([
            "--remote",
            &server.base_url,
            "habit",
            "checkin",
            &habit_uuid.to_string(),
            "--value",
            "30.0",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Check-in recorded for habit"));

    // Scenario 4.1: Archive habit via remote CLI
    let mut archive_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    archive_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args([
            "--remote",
            &server.base_url,
            "habit",
            "archive",
            &habit_uuid.to_string(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("archived"));

    // Scenario 5: Idempotency-Key retry verification
    // Read the stored JWT token to perform duplicate network call with identical idempotency key
    let creds_bytes = fs::read(&creds_path).expect("Read credentials");
    let creds_json: serde_json::Value =
        serde_json::from_slice(&creds_bytes).expect("Valid JSON credentials");
    let token = creds_json["token"].as_str().expect("JWT token");

    let checkin_url = format!("{}/api/v1/habits/{habit_uuid}/checkin", server.base_url);
    let idempotency_key = Uuid::new_v4();

    let today = chrono::Local::now().date_naive();
    let checkin_payload = serde_json::json!({
        "date": today.to_string(),
        "value": 30.0
    });

    // First request with idempotency key
    let first_resp = client
        .post(&checkin_url)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idempotency_key.to_string())
        .json(&checkin_payload)
        .send()
        .await
        .expect("First checkin request");
    assert!(first_resp.status().is_success());
    let first_body: serde_json::Value = first_resp.json().await.expect("JSON response");

    // Replay identical request with identical idempotency key
    let retry_resp = client
        .post(&checkin_url)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idempotency_key.to_string())
        .json(&checkin_payload)
        .send()
        .await
        .expect("Retry checkin request");
    assert!(retry_resp.status().is_success());
    let retry_body: serde_json::Value = retry_resp.json().await.expect("JSON response");

    assert_eq!(
        first_body, retry_body,
        "Idempotent retry must return identical cached response"
    );

    // Scenario 6: Logout removes credentials cleanly
    let mut logout_cmd = Command::cargo_bin("roubit").expect("Binary 'roubit' must exist");
    logout_cmd
        .env("ROUBIT_CREDENTIALS_PATH", creds_path_str)
        .args(["auth", "logout"])
        .assert()
        .success();

    assert!(
        !creds_path.exists(),
        "Credentials file must be deleted upon logout"
    );
}
