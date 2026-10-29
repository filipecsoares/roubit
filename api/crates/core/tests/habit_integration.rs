//! Habit Integration Tests & Use Case Validation (RF02)
//!
//! Validates interactions between domain models, atomic check-in persistence,
//! streak calculation engine, adherence analytics, and SQLite foreign key constraints.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::similar_names,
    clippy::suboptimal_flops
)]

use std::time::Duration;

use chrono::{NaiveDate, Utc};
use roubit_core::domain::{
    create_sqlite_pool, Habit, HabitAnalyticsService, HabitRepository, HabitRepositoryError,
    SqliteHabitRepository, StreakCalculator,
};
use tokio::time::timeout;
use uuid::Uuid;

/// Maximum duration allowed for an async database integration test.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Initializes an isolated in-memory SQLite database repository with all migrations applied.
async fn setup_test_db() -> SqliteHabitRepository {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("pool creation must succeed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations must succeed");

    SqliteHabitRepository::new(pool)
}

#[tokio::test]
async fn test_full_binary_habit_lifecycle_and_streak() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        // 1. Create Daily Binary Habit
        let habit = Habit::new_binary(
            "Morning Meditation",
            Some("10 minutes of mindfulness".into()),
        );
        repo.create_habit(&habit)
            .await
            .expect("create habit must succeed");

        // 2. Perform 5 consecutive daily check-ins
        let today = Utc::now().date_naive();
        for i in 0..5 {
            let date = today - chrono::Duration::days(i);
            let log = repo
                .record_check_in(habit.id, date, 1.0, false)
                .await
                .expect("record check-in must succeed");
            assert!(log.is_completed);
            assert_eq!(log.actual_value, 1.0);
        }

        // 3. Fetch logs and verify streak
        let start_date = today - chrono::Duration::days(10);
        let logs = repo
            .get_logs_for_habit(habit.id, start_date, today)
            .await
            .expect("get logs must succeed");

        assert_eq!(logs.len(), 5);

        let streak = StreakCalculator::calculate_for_habit(&habit, &logs, today);
        assert_eq!(streak.current_streak, 5);
        assert_eq!(streak.longest_streak, 5);
        assert!(streak.is_active_today);

        // 4. Verify adherence analytics
        let summary = HabitAnalyticsService::compute_summary(&habit, &logs, today);
        assert_eq!(summary.total_logs_count, 5);
        // In the 30-day window, 5 of 30 days completed = 16.666668%
        assert!((summary.completion_rate_30d - (5.0 / 30.0 * 100.0)).abs() < 0.1);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_numeric_habit_incremental_checkin_and_heatmaps() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        // 1. Create Numeric Habit (target: 2000 ml water)
        let habit = Habit::new_numeric("Hydration", None, 2000.0, "ml");
        repo.create_habit(&habit)
            .await
            .expect("create habit must succeed");

        let today = Utc::now().date_naive();

        // 2. First drink: 500ml (cumulative: 500 < 2000 -> incomplete)
        let log1 = repo
            .record_check_in(habit.id, today, 500.0, true)
            .await
            .expect("first check-in must succeed");
        assert_eq!(log1.actual_value, 500.0);
        assert!(!log1.is_completed);

        // 3. Second drink: 1000ml (cumulative: 1500 < 2000 -> incomplete)
        let log2 = repo
            .record_check_in(habit.id, today, 1000.0, true)
            .await
            .expect("second check-in must succeed");
        assert_eq!(log2.actual_value, 1500.0);
        assert!(!log2.is_completed);

        // 4. Third drink: 600ml (cumulative: 2100 >= 2000 -> completed!)
        let log3 = repo
            .record_check_in(habit.id, today, 600.0, true)
            .await
            .expect("third check-in must succeed");
        assert_eq!(log3.actual_value, 2100.0);
        assert!(log3.is_completed);

        // 5. Yesterday: single direct overwrite check-in of 2500ml
        let yesterday = today - chrono::Duration::days(1);
        let log_yesterday = repo
            .record_check_in(habit.id, yesterday, 2500.0, false)
            .await
            .expect("yesterday check-in must succeed");
        assert_eq!(log_yesterday.actual_value, 2500.0);
        assert!(log_yesterday.is_completed);

        // 6. Heatmap generation
        let logs = repo
            .get_logs_for_habit(habit.id, yesterday, today)
            .await
            .expect("fetch logs");
        assert_eq!(logs.len(), 2);

        let log_tuples: Vec<(NaiveDate, f64)> =
            logs.iter().map(|l| (l.log_date, l.actual_value)).collect();
        let heatmap = HabitAnalyticsService::generate_heatmap(
            &log_tuples,
            habit.target_value,
            yesterday - chrono::Duration::days(2),
            today,
        );

        // 4 total days in window: [yesterday - 2, yesterday - 1, yesterday, today]
        assert_eq!(heatmap.len(), 4);
        assert_eq!(heatmap[0].intensity, 0); // 0ml
        assert_eq!(heatmap[1].intensity, 0); // 0ml
        assert_eq!(heatmap[2].intensity, 4); // 2500ml >= 2000ml
        assert_eq!(heatmap[3].intensity, 4); // 2100ml >= 2000ml
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_timer_habit_and_weekday_streak_skipping_weekends() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        // 1. Create Timer Habit (target: 90 minutes of study)
        let mut habit = Habit::new_timer("Deep Work", None, 90.0);
        // Mon-Fri bitmask: 1+2+4+8+16 = 31
        habit.frequency_days = 31;
        repo.create_habit(&habit).await.expect("create habit");

        // Specific test calendar:
        // Wednesday: 2026-10-28
        // Thursday:  2026-10-29
        // Friday:    2026-10-30
        // (Saturday 10-31 and Sunday 11-01 skipped)
        // Monday:    2026-11-02
        let wednesday = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let friday = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();

        for date in [wednesday, thursday, friday, monday] {
            repo.record_check_in(habit.id, date, 120.0, false)
                .await
                .expect("check in");
        }

        let logs = repo
            .get_logs_for_habit(habit.id, wednesday, monday)
            .await
            .expect("fetch logs");
        assert_eq!(logs.len(), 4);

        let streak = StreakCalculator::calculate_for_habit(&habit, &logs, monday);
        assert_eq!(streak.current_streak, 4);
        assert_eq!(streak.longest_streak, 4);
        assert!(streak.is_active_today);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_cascade_deletion_removes_logs() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        let habit = Habit::new_binary("Temporary Habit", None);
        repo.create_habit(&habit).await.expect("create habit");

        let today = Utc::now().date_naive();
        repo.record_check_in(habit.id, today, 1.0, false)
            .await
            .expect("check in");

        let logs_before = repo
            .get_logs_for_habit(habit.id, today, today)
            .await
            .expect("fetch logs");
        assert_eq!(logs_before.len(), 1);

        // Delete habit
        repo.delete_habit(habit.id).await.expect("delete habit");

        // Confirm habit is gone
        let fetched = repo.get_habit_by_id(habit.id).await.expect("query");
        assert!(fetched.is_none());

        // Confirm logs were cascade-deleted
        let logs_after = repo
            .get_logs_for_habit(habit.id, today, today)
            .await
            .expect("fetch logs");
        assert_eq!(logs_after.len(), 0);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_habit_archival_and_active_filtering() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        let habit_a = Habit::new_binary("Habit A", None);
        let habit_b = Habit::new_binary("Habit B", None);
        repo.create_habit(&habit_a).await.expect("create A");
        repo.create_habit(&habit_b).await.expect("create B");

        let active_initial = repo.list_active_habits().await.expect("list active");
        assert_eq!(active_initial.len(), 2);

        // Archive habit A
        repo.archive_habit(habit_a.id).await.expect("archive A");

        let active_after = repo.list_active_habits().await.expect("list active");
        assert_eq!(active_after.len(), 1);
        assert_eq!(active_after[0].id, habit_b.id);

        // Direct fetch still returns archived habit A
        let fetched_a = repo
            .get_habit_by_id(habit_a.id)
            .await
            .expect("fetch A")
            .expect("habit A must exist");
        assert!(fetched_a.is_archived);
    })
    .await
    .expect("test timed out");
}

#[tokio::test]
async fn test_not_found_errors() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;
        let random_id = Uuid::new_v4();
        let today = Utc::now().date_naive();

        // 1. Record check-in for non-existent habit
        let check_in_res = repo.record_check_in(random_id, today, 1.0, false).await;
        assert!(matches!(
            check_in_res,
            Err(HabitRepositoryError::NotFound(_))
        ));

        // 2. Archive non-existent habit
        let archive_res = repo.archive_habit(random_id).await;
        assert!(matches!(
            archive_res,
            Err(HabitRepositoryError::NotFound(_))
        ));

        // 3. Delete non-existent habit
        let delete_res = repo.delete_habit(random_id).await;
        assert!(matches!(delete_res, Err(HabitRepositoryError::NotFound(_))));
    })
    .await
    .expect("test timed out");
}
