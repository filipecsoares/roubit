//! Routine SQLite Repository with WAL Mode Support
//!
//! Provides asynchronous, resilient persistence for routines, time blocks,
//! and daily execution logs using `SQLx` and SQLite.

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use super::models::{BlockStatus, DailyRoutineLog, Routine, TimeBlock};

/// Errors occurring during routine repository database operations.
#[derive(Debug, thiserror::Error)]
pub enum RoutineRepositoryError {
    /// Internal `SQLx` driver or connection error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Target routine entity was not found.
    #[error("Routine not found: {0}")]
    NotFound(Uuid),

    /// Target time block entity was not found.
    #[error("Time block not found: {0}")]
    BlockNotFound(Uuid),

    /// Transaction execution failed.
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
}

/// Initializes an asynchronous SQLite connection pool configured for high concurrency:
/// - `journal_mode = WAL` (enables non-blocking concurrent readers while writing)
/// - `synchronous = NORMAL` (high durability with low fsync overhead)
/// - `foreign_keys = ON` (strict relational integrity and cascade deletions)
/// - `busy_timeout = 5s` (mitigates database lock contention under burst writes)
///
/// # Errors
/// Returns [`sqlx::Error`] if connection string parsing or pool connection fails.
pub async fn create_sqlite_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}

/// Abstract contract for routine and time-blocking persistence operations.
#[async_trait]
pub trait RoutineRepository: Send + Sync {
    /// Inserts a new routine template.
    async fn create_routine(&self, routine: &Routine) -> Result<(), RoutineRepositoryError>;

    /// Atomically inserts a routine template alongside its initial time blocks within a single transaction.
    async fn create_routine_with_blocks(
        &self,
        routine: &Routine,
        blocks: &[TimeBlock],
    ) -> Result<(), RoutineRepositoryError>;

    /// Retrieves a routine by its unique identifier.
    async fn get_routine_by_id(&self, id: Uuid) -> Result<Option<Routine>, RoutineRepositoryError>;

    /// Retrieves the currently active routine template (most recently updated active routine).
    async fn get_active_routine(&self) -> Result<Option<Routine>, RoutineRepositoryError>;

    /// Marks a specific routine as active, optionally deactivating all other routines.
    async fn set_active_routine(&self, id: Uuid) -> Result<(), RoutineRepositoryError>;

    /// Appends a new time block to a routine.
    async fn add_time_block(&self, block: &TimeBlock) -> Result<(), RoutineRepositoryError>;

    /// Lists all time blocks associated with a routine, ordered chronologically.
    async fn list_blocks_for_routine(
        &self,
        routine_id: Uuid,
    ) -> Result<Vec<TimeBlock>, RoutineRepositoryError>;

    /// Upserts a daily execution check-in log for a time block on a specific date.
    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
        notes: Option<String>,
    ) -> Result<(), RoutineRepositoryError>;

    /// Retrieves the daily execution log for a specific block and date.
    async fn get_daily_log(
        &self,
        block_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyRoutineLog>, RoutineRepositoryError>;

    /// Deletes a routine and cascades deletion to all associated time blocks and logs.
    async fn delete_routine(&self, id: Uuid) -> Result<(), RoutineRepositoryError>;
}

/// SQLite-backed implementation of [`RoutineRepository`].
#[derive(Debug, Clone)]
pub struct SqliteRoutineRepository {
    pool: SqlitePool,
}

impl SqliteRoutineRepository {
    /// Creates a new `SqliteRoutineRepository` wrapping an active connection pool.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying `SQLx` SQLite connection pool.
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn insert_routine_tx(
        tx: &mut Transaction<'_, Sqlite>,
        routine: &Routine,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO routines (id, name, description, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(routine.id)
        .bind(&routine.name)
        .bind(&routine.description)
        .bind(routine.is_active)
        .bind(routine.created_at)
        .bind(routine.updated_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn insert_block_tx(
        tx: &mut Transaction<'_, Sqlite>,
        block: &TimeBlock,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO time_blocks (id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(block.id)
        .bind(block.routine_id)
        .bind(&block.name)
        .bind(block.start_time)
        .bind(block.end_time)
        .bind(&block.category)
        .bind(&block.color_hex)
        .bind(block.order_index)
        .bind(block.created_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl RoutineRepository for SqliteRoutineRepository {
    async fn create_routine(&self, routine: &Routine) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, routine).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn create_routine_with_blocks(
        &self,
        routine: &Routine,
        blocks: &[TimeBlock],
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, routine).await?;

        for block in blocks {
            Self::insert_block_tx(&mut tx, block).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_routine_by_id(&self, id: Uuid) -> Result<Option<Routine>, RoutineRepositoryError> {
        let routine = sqlx::query_as::<_, Routine>(
            r"
            SELECT id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(routine)
    }

    async fn get_active_routine(&self) -> Result<Option<Routine>, RoutineRepositoryError> {
        let routine = sqlx::query_as::<_, Routine>(
            r"
            SELECT id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE is_active = 1
            ORDER BY updated_at DESC
            LIMIT 1
            ",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(routine)
    }

    async fn set_active_routine(&self, id: Uuid) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // Deactivate all existing routines
        sqlx::query("UPDATE routines SET is_active = 0 WHERE is_active = 1")
            .execute(&mut *tx)
            .await?;

        // Activate selected routine
        let rows_affected = sqlx::query("UPDATE routines SET is_active = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(RoutineRepositoryError::NotFound(id));
        }

        tx.commit().await?;
        Ok(())
    }

    async fn add_time_block(&self, block: &TimeBlock) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_block_tx(&mut tx, block).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_blocks_for_routine(
        &self,
        routine_id: Uuid,
    ) -> Result<Vec<TimeBlock>, RoutineRepositoryError> {
        let blocks = sqlx::query_as::<_, TimeBlock>(
            r"
            SELECT id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at
            FROM time_blocks
            WHERE routine_id = ?
            ORDER BY order_index ASC, start_time ASC
            ",
        )
        .bind(routine_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(blocks)
    }

    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
        notes: Option<String>,
    ) -> Result<(), RoutineRepositoryError> {
        let log_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let started_at = if status == BlockStatus::InProgress {
            Some(now)
        } else {
            None
        };
        let completed_at = if status == BlockStatus::Completed {
            Some(now)
        } else {
            None
        };

        sqlx::query(
            r"
            INSERT INTO daily_routine_logs (id, time_block_id, date, status, started_at, completed_at, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(time_block_id, date) DO UPDATE SET
                status = excluded.status,
                notes = coalesce(excluded.notes, daily_routine_logs.notes),
                started_at = coalesce(daily_routine_logs.started_at, excluded.started_at),
                completed_at = coalesce(excluded.completed_at, daily_routine_logs.completed_at)
            ",
        )
        .bind(log_id)
        .bind(block_id)
        .bind(date)
        .bind(status)
        .bind(started_at)
        .bind(completed_at)
        .bind(notes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_daily_log(
        &self,
        block_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyRoutineLog>, RoutineRepositoryError> {
        let log = sqlx::query_as::<_, DailyRoutineLog>(
            r"
            SELECT id, time_block_id, date, status, started_at, completed_at, notes
            FROM daily_routine_logs
            WHERE time_block_id = ? AND date = ?
            ",
        )
        .bind(block_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    async fn delete_routine(&self, id: Uuid) -> Result<(), RoutineRepositoryError> {
        let rows_affected = sqlx::query("DELETE FROM routines WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(RoutineRepositoryError::NotFound(id));
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    async fn setup_test_repo() -> SqliteRoutineRepository {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("in-memory pool creation must succeed");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations must succeed");

        SqliteRoutineRepository::new(pool)
    }

    #[tokio::test]
    async fn test_sqlite_routine_crud() {
        let repo = setup_test_repo().await;

        let routine = Routine::new("Morning Focus", Some("High performance morning".into()));
        repo.create_routine(&routine)
            .await
            .expect("create routine should succeed");

        let fetched = repo
            .get_routine_by_id(routine.id)
            .await
            .expect("get routine should succeed")
            .expect("routine must be present");

        assert_eq!(fetched.name, "Morning Focus");
        assert!(fetched.is_active);

        // Active routine query
        let active = repo
            .get_active_routine()
            .await
            .expect("get active routine should succeed")
            .expect("active routine must exist");
        assert_eq!(active.id, routine.id);
    }

    #[tokio::test]
    async fn test_sqlite_transactional_create_routine_with_blocks() {
        let repo = setup_test_repo().await;

        let routine = Routine::new("Full Day Flow", None);
        let start1 = NaiveTime::from_hms_opt(8, 0, 0).expect("valid time");
        let end1 = NaiveTime::from_hms_opt(9, 30, 0).expect("valid time");
        let start2 = NaiveTime::from_hms_opt(10, 0, 0).expect("valid time");
        let end2 = NaiveTime::from_hms_opt(12, 0, 0).expect("valid time");

        let blocks = vec![
            TimeBlock::new(routine.id, "Morning Reading", start1, end1, 1),
            TimeBlock::new(routine.id, "Deep Work", start2, end2, 2),
        ];

        repo.create_routine_with_blocks(&routine, &blocks)
            .await
            .expect("transactional creation should succeed");

        let fetched_blocks = repo
            .list_blocks_for_routine(routine.id)
            .await
            .expect("list blocks should succeed");

        assert_eq!(fetched_blocks.len(), 2);
        assert_eq!(fetched_blocks[0].name, "Morning Reading");
        assert_eq!(fetched_blocks[1].name, "Deep Work");
    }

    #[tokio::test]
    async fn test_sqlite_log_block_status_upsert_idempotence() {
        let repo = setup_test_repo().await;

        let routine = Routine::new("Evening Winddown", None);
        let start = NaiveTime::from_hms_opt(21, 0, 0).expect("valid time");
        let end = NaiveTime::from_hms_opt(22, 0, 0).expect("valid time");
        let block = TimeBlock::new(routine.id, "Meditation", start, end, 1);

        repo.create_routine(&routine).await.expect("routine insert");
        repo.add_time_block(&block).await.expect("block insert");

        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");

        // 1. Log In Progress
        repo.log_block_status(
            block.id,
            date,
            BlockStatus::InProgress,
            Some("Starting session".into()),
        )
        .await
        .expect("log in progress");

        let log = repo
            .get_daily_log(block.id, date)
            .await
            .expect("get log")
            .expect("log exists");
        assert_eq!(log.status, BlockStatus::InProgress);
        assert_eq!(log.notes.as_deref(), Some("Starting session"));
        assert!(log.started_at.is_some());

        // 2. Upsert to Completed (same block and date)
        repo.log_block_status(
            block.id,
            date,
            BlockStatus::Completed,
            Some("Finished peacefully".into()),
        )
        .await
        .expect("log completed");

        let updated_log = repo
            .get_daily_log(block.id, date)
            .await
            .expect("get log")
            .expect("log exists");
        assert_eq!(updated_log.status, BlockStatus::Completed);
        assert_eq!(updated_log.notes.as_deref(), Some("Finished peacefully"));
        assert!(updated_log.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_sqlite_cascade_delete() {
        let repo = setup_test_repo().await;

        let routine = Routine::new("Temporary Routine", None);
        let start = NaiveTime::from_hms_opt(14, 0, 0).expect("valid time");
        let end = NaiveTime::from_hms_opt(15, 0, 0).expect("valid time");
        let block = TimeBlock::new(routine.id, "Quick Sync", start, end, 1);

        repo.create_routine_with_blocks(&routine, &[block])
            .await
            .expect("insert");

        let blocks_before = repo
            .list_blocks_for_routine(routine.id)
            .await
            .expect("list");
        assert_eq!(blocks_before.len(), 1);

        // Delete parent routine
        repo.delete_routine(routine.id).await.expect("delete");

        let blocks_after = repo
            .list_blocks_for_routine(routine.id)
            .await
            .expect("list");
        assert_eq!(blocks_after.len(), 0);

        let routine_opt = repo.get_routine_by_id(routine.id).await.expect("get");
        assert!(routine_opt.is_none());
    }
}
