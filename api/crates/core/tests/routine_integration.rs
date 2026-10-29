//! Routine Integration Tests & Use Case Validation (RF01)
//!
//! Validates interactions between domain models, validation rules,
//! progress tracking engine, and the SQLite WAL persistence layer.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use std::time::Duration;

use chrono::{NaiveDate, NaiveTime};
use roubit_core::domain::{
    create_sqlite_pool, BlockStatus, Routine, RoutineProgressTracker, RoutineRepository,
    SqliteRoutineRepository, TimeBlock, TimeBlockValidationError, TimeBlockValidator,
};
use tokio::time::timeout;
use uuid::Uuid;

/// Maximum duration allowed for an async database integration test.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Initializes an isolated in-memory SQLite database repository with all migrations applied.
async fn setup_test_db() -> SqliteRoutineRepository {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("pool creation must succeed");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations must succeed");

    SqliteRoutineRepository::new(pool)
}

fn create_time_block(
    routine_id: Uuid,
    name: &str,
    start: &str,
    end: &str,
    order: i32,
) -> TimeBlock {
    TimeBlock::new(
        routine_id,
        name,
        NaiveTime::parse_from_str(start, "%H:%M:%S").expect("valid start time"),
        NaiveTime::parse_from_str(end, "%H:%M:%S").expect("valid end time"),
        order,
    )
}

#[tokio::test]
async fn test_routine_full_lifecycle_and_adherence_reporting() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        // 1. Create Routine entity
        let routine = Routine::new("Workday Routine", Some("Focus day template".into()));
        let routine_id = routine.id;

        // 2. Define 4 consecutive time blocks
        let blocks = vec![
            create_time_block(routine_id, "Morning Planning", "08:00:00", "09:00:00", 1),
            create_time_block(routine_id, "Deep Work Sprint", "09:00:00", "11:30:00", 2),
            create_time_block(routine_id, "Lunch & Recharge", "11:30:00", "12:30:00", 3),
            create_time_block(
                routine_id,
                "Async Admin & Wrapup",
                "12:30:00",
                "14:00:00",
                4,
            ),
        ];

        // 3. Pre-persistence domain validation
        TimeBlockValidator::validate_schedule(&blocks)
            .expect("schedule validation must pass for consecutive blocks");

        // 4. Atomic transaction persistence
        repo.create_routine_with_blocks(&routine, &blocks)
            .await
            .expect("routine creation with blocks must succeed");

        // 5. Verify persisted blocks
        let fetched_blocks = repo
            .list_blocks_for_routine(routine_id)
            .await
            .expect("list blocks must succeed");
        assert_eq!(fetched_blocks.len(), 4);
        assert_eq!(fetched_blocks[0].name, "Morning Planning");
        assert_eq!(fetched_blocks[1].name, "Deep Work Sprint");

        // 6. Complete Block 1 for today
        let target_date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");
        let block_1_id = fetched_blocks[0].id;

        repo.log_block_status(
            block_1_id,
            target_date,
            BlockStatus::Completed,
            Some("Completed morning planning smoothly".into()),
        )
        .await
        .expect("log block status must succeed");

        let log_1 = repo
            .get_daily_log(block_1_id, target_date)
            .await
            .expect("fetch log must succeed")
            .expect("log entry must exist");

        assert_eq!(log_1.status, BlockStatus::Completed);
        assert_eq!(
            log_1.notes.as_deref(),
            Some("Completed morning planning smoothly")
        );
        assert!(log_1.completed_at.is_some());

        // 7. Evaluate Real-Time Routine Progress at 10:00:00 (during Deep Work Sprint)
        let block_statuses = vec![
            (fetched_blocks[0].clone(), BlockStatus::Completed),
            (fetched_blocks[1].clone(), BlockStatus::Pending),
            (fetched_blocks[2].clone(), BlockStatus::Pending),
            (fetched_blocks[3].clone(), BlockStatus::Pending),
        ];

        let evaluation_time = NaiveTime::from_hms_opt(10, 0, 0).expect("valid time");
        let progress = RoutineProgressTracker::compute_progress(
            &fetched_blocks,
            &block_statuses,
            evaluation_time,
        );

        assert_eq!(progress.total_blocks, 4);
        assert_eq!(progress.completed_blocks, 1);
        assert_eq!(progress.pending_blocks, 3);
        assert_eq!(progress.skipped_blocks, 0);
        assert_eq!(progress.adherence_percentage, 25.0);
        assert_eq!(
            progress.active_block.as_ref().map(|b| b.name.as_str()),
            Some("Deep Work Sprint")
        );
        assert_eq!(
            progress.next_block.as_ref().map(|b| b.name.as_str()),
            Some("Lunch & Recharge")
        );
        // From 10:00:00 to 11:30:00 is 90 minutes
        assert_eq!(progress.minutes_until_next, Some(90));
    })
    .await
    .expect("test execution completed within timeout");
}

#[tokio::test]
async fn test_conflict_prevention_and_overlap_rejection() {
    timeout(TEST_TIMEOUT, async {
        let routine_id = Uuid::new_v4();

        // 1. Overlapping blocks: 08:00-09:30 and 09:00-10:30 (30-min collision)
        let b1 = create_time_block(routine_id, "Task Alpha", "08:00:00", "09:30:00", 1);
        let b2 = create_time_block(routine_id, "Task Beta", "09:00:00", "10:30:00", 2);

        let result = TimeBlockValidator::validate_schedule(&[b1, b2]);
        assert!(matches!(
            result,
            Err(TimeBlockValidationError::OverlapDetected { .. })
        ));

        // 2. Duration below minimum threshold (< 5 minutes)
        let b_short = create_time_block(routine_id, "Flash Check", "08:00:00", "08:03:00", 1);
        let short_result = TimeBlockValidator::validate_schedule(&[b_short]);
        assert_eq!(
            short_result,
            Err(TimeBlockValidationError::DurationTooShort { minutes: 3 })
        );

        // 3. Inverted time interval (start > end)
        let b_inverted = create_time_block(routine_id, "Inverted", "10:00:00", "09:00:00", 1);
        let inverted_result = TimeBlockValidator::validate_schedule(&[b_inverted]);
        assert!(matches!(
            inverted_result,
            Err(TimeBlockValidationError::InvalidInterval { .. })
        ));
    })
    .await
    .expect("test execution completed within timeout");
}

#[tokio::test]
async fn test_cascade_deletion_removes_blocks_and_daily_logs() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        let routine = Routine::new("Temporary Routine", None);
        let routine_id = routine.id;

        let blocks = vec![
            create_time_block(routine_id, "Block One", "08:00:00", "09:00:00", 1),
            create_time_block(routine_id, "Block Two", "09:00:00", "10:00:00", 2),
        ];

        repo.create_routine_with_blocks(&routine, &blocks)
            .await
            .expect("routine creation must succeed");

        let block_1_id = blocks[0].id;
        let block_2_id = blocks[1].id;
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");

        // Log execution for both blocks
        repo.log_block_status(block_1_id, date, BlockStatus::Completed, None)
            .await
            .expect("log block 1 must succeed");
        repo.log_block_status(block_2_id, date, BlockStatus::Skipped, None)
            .await
            .expect("log block 2 must succeed");

        // Verify records exist before cascade
        let initial_blocks = repo
            .list_blocks_for_routine(routine_id)
            .await
            .expect("list blocks succeeds");
        assert_eq!(initial_blocks.len(), 2);

        // Delete parent routine
        repo.delete_routine(routine_id)
            .await
            .expect("delete routine succeeds");

        // 1. Verify routine is gone
        let fetched_routine = repo
            .get_routine_by_id(routine_id)
            .await
            .expect("fetch succeeds");
        assert!(fetched_routine.is_none());

        // 2. Verify associated blocks are gone
        let remaining_blocks = repo
            .list_blocks_for_routine(routine_id)
            .await
            .expect("list blocks succeeds");
        assert_eq!(remaining_blocks.len(), 0);

        // 3. Verify associated daily routine logs were cascaded
        let logs_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM daily_routine_logs WHERE time_block_id IN (?, ?)")
                .bind(block_1_id)
                .bind(block_2_id)
                .fetch_one(repo.pool())
                .await
                .expect("count daily logs must succeed");

        assert_eq!(
            logs_count.0, 0,
            "Daily routine logs must be cascaded on routine deletion"
        );
    })
    .await
    .expect("test execution completed within timeout");
}

#[tokio::test]
async fn test_idempotent_daily_routine_logging_and_status_transitions() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        let routine = Routine::new("Daily Standup Routine", None);
        let routine_id = routine.id;
        let blocks = vec![create_time_block(
            routine_id, "Standup", "09:00:00", "09:30:00", 1,
        )];

        repo.create_routine_with_blocks(&routine, &blocks)
            .await
            .expect("routine creation succeeds");

        let block_id = blocks[0].id;
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");

        // 1. Start execution -> InProgress
        repo.log_block_status(
            block_id,
            date,
            BlockStatus::InProgress,
            Some("Started standup discussion".into()),
        )
        .await
        .expect("log in_progress succeeds");

        let log_in_progress = repo
            .get_daily_log(block_id, date)
            .await
            .expect("fetch log succeeds")
            .expect("log entry exists");

        assert_eq!(log_in_progress.status, BlockStatus::InProgress);
        assert!(log_in_progress.started_at.is_some());
        assert!(log_in_progress.completed_at.is_none());
        assert_eq!(
            log_in_progress.notes.as_deref(),
            Some("Started standup discussion")
        );

        let original_started_at = log_in_progress.started_at;

        // 2. Complete execution on the same date with updated note (idempotent upsert)
        repo.log_block_status(
            block_id,
            date,
            BlockStatus::Completed,
            Some("Standup concluded with team alignment".into()),
        )
        .await
        .expect("log completed succeeds");

        let log_completed = repo
            .get_daily_log(block_id, date)
            .await
            .expect("fetch log succeeds")
            .expect("log entry exists");

        assert_eq!(log_completed.status, BlockStatus::Completed);
        assert_eq!(
            log_completed.started_at, original_started_at,
            "started_at must be preserved across status upserts"
        );
        assert!(log_completed.completed_at.is_some());
        assert_eq!(
            log_completed.notes.as_deref(),
            Some("Standup concluded with team alignment")
        );

        // 3. Ensure database contains exactly 1 log record for (block_id, date)
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM daily_routine_logs WHERE time_block_id = ? AND date = ?",
        )
        .bind(block_id)
        .bind(date)
        .fetch_one(repo.pool())
        .await
        .expect("count query succeeds");

        assert_eq!(
            count.0, 1,
            "Exactly one log record must exist per block/date"
        );
    })
    .await
    .expect("test execution completed within timeout");
}

#[tokio::test]
async fn test_active_routine_switching() {
    timeout(TEST_TIMEOUT, async {
        let repo = setup_test_db().await;

        let routine_a = Routine::new("Routine A", None);
        let routine_b = Routine::new("Routine B", None);

        repo.create_routine(&routine_a).await.expect("create A");
        repo.create_routine(&routine_b).await.expect("create B");

        // Activate Routine A exclusively
        repo.set_active_routine(routine_a.id)
            .await
            .expect("set active A");

        let active = repo
            .get_active_routine()
            .await
            .expect("get active")
            .expect("active exists");
        assert_eq!(active.id, routine_a.id);

        let fetched_b = repo
            .get_routine_by_id(routine_b.id)
            .await
            .expect("fetch B")
            .expect("B exists");
        assert!(!fetched_b.is_active, "Routine B should be deactivated");
    })
    .await
    .expect("test execution completed within timeout");
}
