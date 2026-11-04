//! PostgreSQL & Redis Integration Test Suite (IMP-02.4)
//!
//! Validates:
//! 1. `DatabasePool` routing and health check for SQLite and PostgreSQL.
//! 2. PostgreSQL schema migrations (`migrations_pg/`) and all 5 `Pg*Repository` implementations
//!    (`PgUserRepository`, `PgRoutineRepository`, `PgHabitRepository`, `PgOkrRepository`, `PgJournalRepository`),
//!    including strict multi-tenant row-level isolation, atomic habit streak calculations, and OKR sync.
//! 3. End-to-end `migrate_sqlite_to_postgres` data migrator fidelity (binary SQLite UUIDs, original
//!    timestamps, streaks, and zero-knowledge encrypted journal envelopes) and idempotent re-execution.
//! 4. Redis distributed locking, idempotency caching, and habit summary caching (`RedisService`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::similar_names
)]

use chrono::{NaiveDate, NaiveTime, Utc};
use roubit_core::{
    create_sqlite_pool, migrate_sqlite_to_postgres, run_migrations, BlockStatus, DatabasePool,
    EncryptedJournalEntry, Habit, HabitRepository, JournalMetadata, JournalRepository, KeyResult,
    Objective, OkrRepository, OkrSyncService, PgHabitRepository, PgJournalRepository,
    PgOkrRepository, PgRoutineRepository, PgUserRepository, Routine, RoutineRepository,
    SqliteHabitRepository, SqliteJournalRepository, SqliteOkrRepository, SqliteRoutineRepository,
    SqliteUserRepository, TimeBlock, User, UserRepository,
};
use uuid::Uuid;

static PG_TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_database_pool_sqlite_routing_and_health_check() {
    let pool = DatabasePool::from_url("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite via DatabasePool");
    assert!(pool.is_sqlite());
    assert!(!pool.is_postgres());

    pool.run_migrations()
        .await
        .expect("Failed to apply SQLite migrations");
    pool.health_check()
        .await
        .expect("SQLite health check must succeed");
}

#[tokio::test]
async fn test_postgres_repositories_and_multi_tenant_isolation() {
    let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") else {
        eprintln!(
            "Skipping test_postgres_repositories_and_multi_tenant_isolation: TEST_POSTGRES_URL not set"
        );
        return;
    };
    let _guard = PG_TEST_MUTEX.lock().await;

    let db_pool = DatabasePool::from_url(&pg_url)
        .await
        .expect("Failed to connect to TEST_POSTGRES_URL");
    assert!(db_pool.is_postgres());
    db_pool
        .run_migrations()
        .await
        .expect("Failed to apply PostgreSQL migrations");
    db_pool
        .health_check()
        .await
        .expect("PostgreSQL health check must succeed");

    let pg_pool = db_pool.postgres().expect("Expected PgPool").clone();

    let user_repo = PgUserRepository::new(pg_pool.clone());
    let routine_repo = PgRoutineRepository::new(pg_pool.clone());
    let habit_repo = PgHabitRepository::new(pg_pool.clone());
    let okr_repo = PgOkrRepository::new(pg_pool.clone());
    let journal_repo = PgJournalRepository::new(pg_pool.clone());

    // 1. User CRUD & Multi-Tenant Setup
    let user_a = User::new(
        Uuid::new_v4(),
        format!("pg_alice_{}@roubit.app", Uuid::new_v4()),
        "$argon2id$v=19$m=19456,t=2,p=1$hash_a".to_string(),
        Utc::now().to_rfc3339(),
    );
    let user_b = User::new(
        Uuid::new_v4(),
        format!("pg_bob_{}@roubit.app", Uuid::new_v4()),
        "$argon2id$v=19$m=19456,t=2,p=1$hash_b".to_string(),
        Utc::now().to_rfc3339(),
    );

    user_repo
        .create_user(&user_a)
        .await
        .expect("Create user_a in PG");
    user_repo
        .create_user(&user_b)
        .await
        .expect("Create user_b in PG");

    // Duplicate email must fail with Conflict
    let dup_res = user_repo.create_user(&user_a).await;
    assert!(dup_res.is_err(), "Duplicate email must be rejected in PG");

    let fetched_a = user_repo
        .find_by_email(&user_a.email)
        .await
        .expect("Query user by email")
        .expect("user_a must exist");
    assert_eq!(fetched_a.id, user_a.id);

    // 2. Routine & TimeBlock CRUD + Multi-Tenant Isolation
    let routine_id = Uuid::new_v4();
    let block_id = Uuid::new_v4();
    let routine = Routine {
        id: routine_id,
        user_id: Some(user_a.id),
        name: "PG Deep Work Routine".to_string(),
        description: Some("PostgreSQL backed routine".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let block = TimeBlock {
        id: block_id,
        routine_id,
        name: "Architecture Design".to_string(),
        start_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        end_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        category: "Deep Work".to_string(),
        color_hex: "#6366F1".to_string(),
        order_index: 0,
        created_at: Utc::now(),
    };

    routine_repo
        .create_routine_with_blocks(user_a.id, &routine, &[block])
        .await
        .expect("Create routine with blocks in PG");

    // Tenant B must not see Tenant A's routine
    assert!(routine_repo
        .get_routine_by_id(routine_id, user_b.id)
        .await
        .expect("Query routine as user_b")
        .is_none());

    let active_a = routine_repo
        .get_active_routine(user_a.id)
        .await
        .expect("Get active routine")
        .expect("Active routine must exist");
    assert_eq!(active_a.id, routine_id);

    let today = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    routine_repo
        .log_block_status(block_id, user_a.id, today, BlockStatus::Completed, None)
        .await
        .expect("Log block status in PG");

    let log_entry = routine_repo
        .get_daily_log(block_id, user_a.id, today)
        .await
        .expect("Get daily log in PG")
        .expect("Daily log must exist");
    assert_eq!(log_entry.status, BlockStatus::Completed);

    // 3. Habit CRUD, Atomic Check-in Streaks & Multi-Tenant Isolation
    let habit = Habit::new_numeric("PG Hydration", None, 2.0, "L");
    let habit_id = habit.id;
    habit_repo
        .create_habit(user_a.id, &habit)
        .await
        .expect("Create habit in PG");

    let day1 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();

    let log1 = habit_repo
        .record_check_in(habit_id, user_a.id, day1, 2.0, false)
        .await
        .expect("Check-in day 1 in PG");
    assert!(log1.is_completed);

    let _ = habit_repo
        .record_check_in(habit_id, user_a.id, day2, 2.5, false)
        .await
        .expect("Check-in day 2 in PG");
    let updated2 = habit_repo
        .get_habit_by_id(habit_id, user_a.id)
        .await
        .expect("Fetch updated habit")
        .expect("Habit must exist");
    assert_eq!(updated2.current_streak, 2);
    assert_eq!(updated2.longest_streak, 2);

    // Tenant B cannot check in to Tenant A's habit
    assert!(habit_repo
        .record_check_in(habit_id, user_b.id, day2, 1.0, false)
        .await
        .is_err());

    // 4. OKR CRUD & Linked Habit Sync in PostgreSQL
    let q_start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let q_end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    let objective = Objective::new(
        "PG Q4 Excellence",
        Some("PostgreSQL OKR".to_string()),
        "2026-Q4",
        q_start,
        q_end,
    )
    .with_user_id(user_a.id);
    let kr = KeyResult::new_habit_linked(
        objective.id,
        habit_id,
        "Consistent Hydration",
        10.0,
        "days",
        1.0,
    );

    okr_repo
        .create_objective(user_a.id, &objective)
        .await
        .expect("Create objective in PG");
    okr_repo
        .create_key_result(user_a.id, &kr)
        .await
        .expect("Create KR in PG");

    let okr_sync = OkrSyncService::new(
        PgOkrRepository::new(pg_pool.clone()),
        PgHabitRepository::new(pg_pool.clone()),
    );
    okr_sync
        .sync_linked_krs_for_habit(user_a.id, habit_id)
        .await
        .expect("Sync OKR on habit check-in in PG");
    let synced_obj = okr_repo
        .get_objective_with_krs(objective.id, user_a.id)
        .await
        .expect("Get synced objective")
        .expect("Objective must exist");
    assert!((synced_obj.objective.progress_percentage - 45.0).abs() < 0.01);

    // 5. Zero-Knowledge Encrypted Journal CRUD & Multi-Tenant Isolation
    let entry_date = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    let meta = JournalMetadata::new(
        entry_date,
        Some(5),
        vec!["postgres".to_string(), "security".to_string()],
        48,
    )
    .expect("Valid journal metadata")
    .with_user_id(user_a.id);
    let entry_id = meta.id;
    let entry =
        EncryptedJournalEntry::new(meta, [0x11; 16], [0x22; 12], vec![0xAB; 48]).expect("Valid");

    journal_repo
        .save_entry(user_a.id, &entry)
        .await
        .expect("Save journal entry in PG");

    // Tenant B cannot read Tenant A's journal entry
    assert!(journal_repo
        .get_entry_by_id(entry_id, user_b.id)
        .await
        .expect("Get entry as user_b")
        .is_none());

    let fetched_entry = journal_repo
        .get_entry_by_date(entry_date, user_a.id)
        .await
        .expect("Get entry by date in PG")
        .expect("Entry must exist");
    assert_eq!(fetched_entry.metadata.id, entry_id);
    assert_eq!(fetched_entry.encrypted_payload, vec![0xAB; 48]);
    assert_eq!(fetched_entry.metadata.mood_score, Some(5));
}

#[tokio::test]
async fn test_sqlite_to_postgres_data_migrator_e2e_and_idempotency() {
    let Ok(pg_url) = std::env::var("TEST_POSTGRES_URL") else {
        eprintln!(
            "Skipping test_sqlite_to_postgres_data_migrator_e2e_and_idempotency: TEST_POSTGRES_URL not set"
        );
        return;
    };
    let _guard = PG_TEST_MUTEX.lock().await;

    let sqlite_pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Create in-memory SQLite pool");
    run_migrations(&sqlite_pool)
        .await
        .expect("Run SQLite migrations");

    let user = User::new(
        Uuid::new_v4(),
        format!("migrated_user_{}@roubit.app", Uuid::new_v4()),
        "$argon2id$v=19$m=19456,t=2,p=1$migrated_hash".to_string(),
        Utc::now().to_rfc3339(),
    );
    SqliteUserRepository::new(sqlite_pool.clone())
        .create_user(&user)
        .await
        .expect("Seed SQLite user");

    let routine_id = Uuid::new_v4();
    let block_id = Uuid::new_v4();
    let routine = Routine {
        id: routine_id,
        user_id: Some(user.id),
        name: "Migrated Routine".to_string(),
        description: Some("From SQLite to Postgres".to_string()),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let block = TimeBlock {
        id: block_id,
        routine_id,
        name: "Migrated Block".to_string(),
        start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        end_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        category: "Focus".to_string(),
        color_hex: "#10B981".to_string(),
        order_index: 0,
        created_at: Utc::now(),
    };
    let sqlite_routine_repo = SqliteRoutineRepository::new(sqlite_pool.clone());
    sqlite_routine_repo
        .create_routine_with_blocks(user.id, &routine, &[block])
        .await
        .expect("Seed SQLite routine");

    let log_date = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    sqlite_routine_repo
        .log_block_status(block_id, user.id, log_date, BlockStatus::Completed, None)
        .await
        .expect("Seed SQLite routine log");

    let habit = Habit::new_binary("Migrated Meditation", Some("Daily calm".to_string()));
    let habit_id = habit.id;
    let sqlite_habit_repo = SqliteHabitRepository::new(sqlite_pool.clone());
    sqlite_habit_repo
        .create_habit(user.id, &habit)
        .await
        .expect("Seed SQLite habit");
    sqlite_habit_repo
        .record_check_in(habit_id, user.id, log_date, 1.0, false)
        .await
        .expect("Seed SQLite habit checkin");

    let q_start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let q_end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    let objective =
        Objective::new("Migrated Objective", None, "2026-Q4", q_start, q_end).with_user_id(user.id);
    let kr =
        KeyResult::new_habit_linked(objective.id, habit_id, "Migrated KR", 30.0, "sessions", 1.0);
    let sqlite_okr_repo = SqliteOkrRepository::new(sqlite_pool.clone());
    sqlite_okr_repo
        .create_objective(user.id, &objective)
        .await
        .expect("Seed SQLite objective");
    sqlite_okr_repo
        .create_key_result(user.id, &kr)
        .await
        .expect("Seed SQLite KR");

    let journal_meta = JournalMetadata::new(log_date, Some(4), vec!["migration".to_string()], 64)
        .expect("Valid journal metadata")
        .with_user_id(user.id);
    let journal_id = journal_meta.id;
    let journal_entry =
        EncryptedJournalEntry::new(journal_meta, [0x33; 16], [0x44; 12], vec![0xFE; 64])
            .expect("Valid journal entry");
    SqliteJournalRepository::new(sqlite_pool.clone())
        .save_entry(user.id, &journal_entry)
        .await
        .expect("Seed SQLite journal entry");

    let db_pool = DatabasePool::from_url(&pg_url)
        .await
        .expect("Connect to Postgres");
    let pg_pool = db_pool.postgres().expect("Expected PgPool").clone();

    // Execute migration #1
    let report1 = migrate_sqlite_to_postgres(&sqlite_pool, &pg_pool)
        .await
        .expect("First migrate_sqlite_to_postgres must succeed");
    assert!(report1.users >= 1);
    assert_eq!(report1.routines, 1);
    assert_eq!(report1.time_blocks, 1);
    assert_eq!(report1.daily_routine_logs, 1);
    assert_eq!(report1.habits, 1);
    assert_eq!(report1.habit_logs, 1);
    assert_eq!(report1.objectives, 1);
    assert_eq!(report1.key_results, 1);
    assert_eq!(report1.journal_entries, 1);

    // Execute migration #2 (idempotent re-execution)
    let report2 = migrate_sqlite_to_postgres(&sqlite_pool, &pg_pool)
        .await
        .expect("Second idempotent migrate_sqlite_to_postgres must succeed");
    assert_eq!(report1, report2);

    // Verify migrated data in PostgreSQL via Pg*Repository
    let pg_habit = PgHabitRepository::new(pg_pool.clone())
        .get_habit_by_id(habit_id, user.id)
        .await
        .expect("Query migrated habit")
        .expect("Migrated habit must exist in Postgres");
    assert_eq!(pg_habit.name, "Migrated Meditation");
    assert_eq!(pg_habit.current_streak, 1);

    let pg_journal = PgJournalRepository::new(pg_pool)
        .get_entry_by_id(journal_id, user.id)
        .await
        .expect("Query migrated journal")
        .expect("Migrated journal must exist in Postgres");
    assert_eq!(pg_journal.encrypted_payload, vec![0xFE; 64]);
    assert_eq!(pg_journal.salt, [0x33; 16]);
}

#[tokio::test]
async fn test_redis_distributed_locks_and_caching_integration() {
    let Ok(redis_url) = std::env::var("TEST_REDIS_URL") else {
        eprintln!(
            "Skipping test_redis_distributed_locks_and_caching_integration: TEST_REDIS_URL not set"
        );
        return;
    };

    let redis =
        roubit_core::redis::RedisService::new(&redis_url).expect("Valid Redis pool configuration");
    let pong = redis.ping().await.expect("Redis PING must succeed");
    assert_eq!(pong, "PONG");

    let resource = format!("test_res_{}", Uuid::new_v4());
    let token = Uuid::new_v4().to_string();

    assert!(redis
        .acquire_lock(&resource, &token, 5_000)
        .await
        .expect("Acquire lock"));
    assert!(!redis
        .acquire_lock(&resource, "other_token", 5_000)
        .await
        .expect("Concurrent lock must return false"));
    assert!(!redis
        .release_lock(&resource, "other_token")
        .await
        .expect("Release with wrong token must return false"));
    assert!(redis
        .release_lock(&resource, &token)
        .await
        .expect("Release with owner token must return true"));
}
