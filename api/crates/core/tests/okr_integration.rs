//! OKR Integration Tests & Cascade Validation (RF03)
//!
//! Validates interactions between domain models, OKR repository,
//! habit-to-KR progress engine, and SQLite foreign key cascading rules.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

use std::time::Duration;

use chrono::NaiveDate;
use roubit_core::domain::{
    create_sqlite_pool, Habit, HabitRepository, KeyResult, Objective, OkrRepository,
    OkrRepositoryError, OkrSyncService, SqliteHabitRepository, SqliteOkrRepository,
};
use tokio::time::timeout;
use uuid::Uuid;

/// Maximum duration allowed for an async database integration test.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Initializes an isolated in-memory SQLite database repository with all migrations applied.
async fn setup_test_repos() -> (SqliteOkrRepository, SqliteHabitRepository, sqlx::SqlitePool) {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("pool creation must succeed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations must succeed");

    (
        SqliteOkrRepository::new(pool.clone()),
        SqliteHabitRepository::new(pool.clone()),
        pool,
    )
}

#[tokio::test]
async fn test_cascade_delete_objective_removes_krs() {
    timeout(TEST_TIMEOUT, async {
        let (okr_repo, _, pool) = setup_test_repos().await;

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Learn Rust Internals", None, "2026-Q4", start, end);
        okr_repo
            .create_objective(&obj)
            .await
            .expect("create objective");

        let kr1 = KeyResult::new_numeric(obj.id, "Read The Rustonomicon", 0.0, 1.0, "book", 1.0);
        let kr2 =
            KeyResult::new_numeric(obj.id, "Write 3 unsafe modules", 0.0, 3.0, "modules", 1.0);
        okr_repo.create_key_result(&kr1).await.expect("create kr1");
        okr_repo.create_key_result(&kr2).await.expect("create kr2");

        // Verify key results exist
        let loaded = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .expect("obj exists");
        assert_eq!(loaded.key_results.len(), 2);

        // Delete parent objective
        okr_repo.delete_objective(obj.id).await.expect("delete obj");

        // Verify objective is gone
        let loaded_after = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch");
        assert!(loaded_after.is_none());

        // Verify child Key Results were cascade deleted from SQLite
        let remaining_krs: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM key_results WHERE objective_id = ?")
                .bind(obj.id)
                .fetch_one(&pool)
                .await
                .expect("count query");

        assert_eq!(remaining_krs, 0);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_habit_deletion_sets_null_on_linked_habit_id() {
    timeout(TEST_TIMEOUT, async {
        let (okr_repo, habit_repo, pool) = setup_test_repos().await;

        // 1. Create a habit
        let habit = Habit::new_numeric("Daily Pushups", None, 50.0, "reps");
        habit_repo.create_habit(&habit).await.expect("create habit");

        // 2. Create objective and linked KR
        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Upper Body Strength", None, "2026-Q4", start, end);
        okr_repo
            .create_objective(&obj)
            .await
            .expect("create objective");

        let kr = KeyResult::new_habit_linked(
            obj.id,
            habit.id,
            "Complete 3000 Pushups",
            3000.0,
            "reps",
            1.0,
        );
        okr_repo.create_key_result(&kr).await.expect("create kr");

        // Verify KR is linked to habit
        let loaded = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .expect("obj exists");
        assert_eq!(loaded.key_results[0].linked_habit_id, Some(habit.id));

        // 3. Delete the habit
        habit_repo
            .delete_habit(habit.id)
            .await
            .expect("delete habit");

        // 4. Verify the Key Result STILL exists, but its linked_habit_id was set to NULL
        let loaded_after = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .expect("obj exists");
        assert_eq!(loaded_after.key_results.len(), 1);
        assert_eq!(loaded_after.key_results[0].id, kr.id);
        assert_eq!(loaded_after.key_results[0].linked_habit_id, None);

        // Verify directly in database table
        let db_kr = sqlx::query_as::<_, KeyResult>("SELECT * FROM key_results WHERE id = ?")
            .bind(kr.id)
            .fetch_one(&pool)
            .await
            .expect("fetch kr");
        assert_eq!(db_kr.linked_habit_id, None);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_live_progress_propagation_from_habit_checkins() {
    timeout(TEST_TIMEOUT, async {
        let (okr_repo, habit_repo, _) = setup_test_repos().await;
        let sync_service = OkrSyncService::new(okr_repo.clone(), habit_repo.clone());

        // 1. Create two habits:
        // Habit 1: Numeric Running (target: 5.0 km per session)
        // Habit 2: Binary Meditation
        let running_habit = Habit::new_numeric("Running", None, 5.0, "km");
        let meditation_habit = Habit::new_binary("Meditation", None);
        habit_repo
            .create_habit(&running_habit)
            .await
            .expect("create running");
        habit_repo
            .create_habit(&meditation_habit)
            .await
            .expect("create meditation");

        // 2. Create Objective with two linked KRs
        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Q4 Health & Mindfulness", None, "2026-Q4", start, end);
        okr_repo
            .create_objective(&obj)
            .await
            .expect("create objective");

        // KR 1: Run 100km total (weight 2.0)
        let kr_run =
            KeyResult::new_habit_linked(obj.id, running_habit.id, "Run 100 km", 100.0, "km", 2.0);
        // KR 2: 30 days of meditation (weight 1.0)
        let kr_med = KeyResult::new_habit_linked(
            obj.id,
            meditation_habit.id,
            "30 Meditation Sessions",
            30.0,
            "sessions",
            1.0,
        );
        okr_repo
            .create_key_result(&kr_run)
            .await
            .expect("create kr_run");
        okr_repo
            .create_key_result(&kr_med)
            .await
            .expect("create kr_med");

        // Initial progress is 0%
        let loaded = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .unwrap();
        assert_eq!(loaded.objective.progress_percentage, 0.0);
        assert!(!loaded.objective.is_completed);

        // 3. Record habit check-ins:
        // Running: 25km total (25% progress on KR 1)
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
        habit_repo
            .record_check_in(running_habit.id, d1, 15.0, true)
            .await
            .unwrap();
        habit_repo
            .record_check_in(running_habit.id, d2, 10.0, true)
            .await
            .unwrap();

        // Meditation: 15 completed days (50% progress on KR 2)
        for i in 0..15 {
            let d = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap() + chrono::Duration::days(i);
            habit_repo
                .record_check_in(meditation_habit.id, d, 1.0, false)
                .await
                .unwrap();
        }

        // 4. Synchronize running habit
        sync_service
            .sync_linked_krs_for_habit(running_habit.id)
            .await
            .expect("sync running");

        // 5. Synchronize meditation habit
        sync_service
            .sync_linked_krs_for_habit(meditation_habit.id)
            .await
            .expect("sync meditation");

        // 6. Verify progress:
        // KR 1 (run): 25km / 100km = 25.0%
        // KR 2 (med): 15 / 30 = 50.0%
        // Weighted objective: (25% * 2.0 + 50% * 1.0) / 3.0 = 100.0 / 3.0 = 33.333336%
        let loaded_mid = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .unwrap();
        assert_eq!(loaded_mid.key_results[0].current_value, 25.0);
        assert_eq!(loaded_mid.key_results[0].progress_percentage, 25.0);
        assert_eq!(loaded_mid.key_results[1].current_value, 15.0);
        assert_eq!(loaded_mid.key_results[1].progress_percentage, 50.0);
        assert!((loaded_mid.objective.progress_percentage - 33.333336).abs() < 0.01);
        assert!(!loaded_mid.objective.is_completed);

        // 7. Complete the rest of running (another 75km) and meditation (another 15 days)
        let d3 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        habit_repo
            .record_check_in(running_habit.id, d3, 75.0, true)
            .await
            .unwrap();

        for i in 15..30 {
            let d = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap() + chrono::Duration::days(i);
            habit_repo
                .record_check_in(meditation_habit.id, d, 1.0, false)
                .await
                .unwrap();
        }

        sync_service
            .sync_linked_krs_for_habit(running_habit.id)
            .await
            .expect("sync running");
        sync_service
            .sync_linked_krs_for_habit(meditation_habit.id)
            .await
            .expect("sync meditation");

        let loaded_final = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .expect("fetch")
            .unwrap();
        assert_eq!(loaded_final.key_results[0].progress_percentage, 100.0);
        assert_eq!(loaded_final.key_results[1].progress_percentage, 100.0);
        assert_eq!(loaded_final.objective.progress_percentage, 100.0);
        assert!(loaded_final.objective.is_completed);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_quarter_listing_and_not_found_errors() {
    timeout(TEST_TIMEOUT, async {
        let (okr_repo, _, _) = setup_test_repos().await;

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();

        let obj1 = Objective::new("Q4 Main Goal", None, "2026-Q4", start, end);
        let obj2 = Objective::new("Q1 Forward Goal", None, "2027-Q1", start, end);

        okr_repo.create_objective(&obj1).await.expect("create obj1");
        okr_repo.create_objective(&obj2).await.expect("create obj2");

        let q4_list = okr_repo
            .list_objectives_by_quarter("2026-Q4")
            .await
            .expect("list");
        assert_eq!(q4_list.len(), 1);
        assert_eq!(q4_list[0].objective.title, "Q4 Main Goal");

        let random_id = Uuid::new_v4();
        let err_kr = okr_repo.update_key_result_value(random_id, 10.0).await;
        assert!(matches!(
            err_kr,
            Err(OkrRepositoryError::KeyResultNotFound(_))
        ));

        let err_obj = okr_repo.delete_objective(random_id).await;
        assert!(matches!(
            err_obj,
            Err(OkrRepositoryError::ObjectiveNotFound(_))
        ));
    })
    .await
    .expect("test timed out");
}
