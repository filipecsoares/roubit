//! End-to-End (E2E) automated tests for the Roubit CLI executable.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

use assert_cmd::Command;
use chrono::{NaiveDate, NaiveTime, Utc};
use predicates::prelude::*;
use roubit_core::{
    create_sqlite_pool,
    domain::{
        habit::models::{Habit, HabitType},
        okr::models::{KeyResult, Objective},
        routine::models::{Routine, TimeBlock},
        HabitRepository, OkrRepository, RoutineRepository, SqliteHabitRepository,
        SqliteOkrRepository, SqliteRoutineRepository,
    },
    run_migrations,
};
use tempfile::NamedTempFile;
use uuid::Uuid;

#[test]
fn test_cli_help_and_version() {
    let mut cmd_help = Command::cargo_bin("roubit").unwrap();
    cmd_help
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "routine, habit, OKR, and encrypted journaling",
        ));

    let mut cmd_version = Command::cargo_bin("roubit").unwrap();
    cmd_version
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("roubit"));
}

#[test]
fn test_cli_empty_args_and_json_banner() {
    let mut cmd = Command::cargo_bin("roubit").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Roubit CLI v"));

    let mut cmd_json = Command::cargo_bin("roubit").unwrap();
    let assert = cmd_json.arg("--json").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert_eq!(parsed["app"], "roubit");
    assert_eq!(parsed["status"], "ready");
}

#[tokio::test]
async fn test_cli_habit_workflow_in_temp_db() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    // 1. Initialize DB migrations and seed habit
    let pool = create_sqlite_pool(db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let habit_repo = SqliteHabitRepository::new(pool);
    let habit = Habit {
        id: Uuid::new_v4(),
        name: "Hydration".to_string(),
        description: Some("Drink 2000 ml water".to_string()),
        habit_type: HabitType::Numeric,
        target_value: 2000.0,
        unit: Some("ml".to_string()),
        frequency_days: Habit::FREQUENCY_DAILY,
        color_hex: Habit::DEFAULT_COLOR.to_string(),
        is_archived: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    habit_repo.create_habit(&habit).await.unwrap();

    // 2. Perform check-in via CLI binary
    let mut checkin_cmd = Command::cargo_bin("roubit").unwrap();
    checkin_cmd
        .args([
            "--db",
            db_path,
            "habit",
            "checkin",
            &habit.id.to_string(),
            "--value",
            "500",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Check-in recorded for habit"));

    // 3. List habits in JSON format
    let mut list_cmd = Command::cargo_bin("roubit").unwrap();
    let assert = list_cmd
        .args(["--db", db_path, "--json", "habit", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["name"], "Hydration");

    // 4. Inspect habit streaks output table
    let mut streaks_cmd = Command::cargo_bin("roubit").unwrap();
    streaks_cmd
        .args(["--db", db_path, "habit", "streaks"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hydration"));
}

#[tokio::test]
async fn test_cli_routine_workflow_in_temp_db() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let pool = create_sqlite_pool(db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let routine_repo = SqliteRoutineRepository::new(pool);
    let routine = Routine::new("Focus Flow", Some("Deep work schedule".to_string()));
    let block = TimeBlock {
        id: Uuid::new_v4(),
        routine_id: routine.id,
        name: "Morning Coding".to_string(),
        start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        end_time: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        category: "Focus".to_string(),
        color_hex: "#6366F1".to_string(),
        order_index: 0,
        created_at: Utc::now(),
    };
    routine_repo
        .create_routine_with_blocks(&routine, std::slice::from_ref(&block))
        .await
        .unwrap();

    // 1. List routine via CLI
    let mut list_cmd = Command::cargo_bin("roubit").unwrap();
    list_cmd
        .args(["--db", db_path, "routine", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Morning Coding"));

    // 2. Start block
    let mut start_cmd = Command::cargo_bin("roubit").unwrap();
    start_cmd
        .args(["--db", db_path, "routine", "start", &block.id.to_string()])
        .assert()
        .success()
        .stdout(predicate::str::contains("in_progress"));

    // 3. Complete block
    let mut complete_cmd = Command::cargo_bin("roubit").unwrap();
    complete_cmd
        .args([
            "--db",
            db_path,
            "routine",
            "complete",
            &block.id.to_string(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("completed"));

    // 4. JSON output verification
    let mut json_cmd = Command::cargo_bin("roubit").unwrap();
    let assert = json_cmd
        .args(["--db", db_path, "--json", "routine", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert!(parsed["adherence_percentage"].is_number());
}

#[tokio::test]
async fn test_cli_okr_workflow_in_temp_db() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let pool = create_sqlite_pool(db_path).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let okr_repo = SqliteOkrRepository::new(pool);
    let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    let objective = Objective::new("Ship Roubit v1.0", None, "2026-Q4", start, end);
    let kr = KeyResult::new_numeric(
        objective.id,
        "Implement CLI Features",
        0.0,
        5.0,
        "features",
        1.0,
    );
    okr_repo.create_objective(&objective).await.unwrap();
    okr_repo.create_key_result(&kr).await.unwrap();

    // 1. OKR Summary via CLI
    let mut summary_cmd = Command::cargo_bin("roubit").unwrap();
    summary_cmd
        .args(["--db", db_path, "okr", "summary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ship Roubit v1.0"));

    // 2. OKR List with JSON
    let mut list_cmd = Command::cargo_bin("roubit").unwrap();
    let assert = list_cmd
        .args(["--db", db_path, "--json", "okr", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["objective"]["quarter"], "2026-Q4");
}

#[test]
fn test_cli_journal_workflow_in_temp_db() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    // 1. Write encrypted entry
    let mut write_cmd = Command::cargo_bin("roubit").unwrap();
    write_cmd
        .env("ROUBIT_PASSPHRASE", "master-secret-password-123")
        .args([
            "--db",
            db_path,
            "journal",
            "write",
            "--content",
            "Testing encrypted CLI journaling with zero knowledge.",
            "-m",
            "5",
            "-t",
            "rust,e2e",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Encrypted journal entry saved"));

    // 2. List journal metadata as JSON
    let mut list_cmd = Command::cargo_bin("roubit").unwrap();
    let assert = list_cmd
        .env("ROUBIT_PASSPHRASE", "master-secret-password-123")
        .args(["--db", db_path, "--json", "journal", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["mood_score"], 5);

    // 3. Read and decrypt entry
    let mut read_cmd = Command::cargo_bin("roubit").unwrap();
    read_cmd
        .env("ROUBIT_PASSPHRASE", "master-secret-password-123")
        .args(["--db", db_path, "journal", "read"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Testing encrypted CLI journaling"));
}

#[test]
fn test_cli_error_cases_and_invalid_arguments() {
    // Unknown subcommand fails
    let mut cmd_bad = Command::cargo_bin("roubit").unwrap();
    cmd_bad.arg("nonexistent_command").assert().failure();

    // Invalid UUID for habit checkin fails
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();

    let mut cmd_uuid = Command::cargo_bin("roubit").unwrap();
    cmd_uuid
        .args(["--db", db_path, "habit", "checkin", "not-a-valid-uuid"])
        .assert()
        .failure();
}

#[test]
fn test_cli_config_show() {
    let mut cmd = Command::cargo_bin("roubit").unwrap();
    cmd.args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Roubit Configuration:"));

    let mut cmd_json = Command::cargo_bin("roubit").unwrap();
    let assert = cmd_json
        .args(["--json", "config", "show"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Must be valid JSON");
    assert!(parsed["database_url"].is_string());
}
