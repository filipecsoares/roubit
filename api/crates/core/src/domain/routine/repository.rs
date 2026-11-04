//! Routine SQLite Repository with WAL Mode Support
//!
//! Provides asynchronous, resilient persistence for routines, time blocks,
//! and daily execution logs using `SQLx` and SQLite.

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{PgPool, Postgres, Row, Sqlite, SqlitePool, Transaction};
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
/// - `cache_size = -64000` (64MB memory page cache for hot read paths)
/// - `temp_store = MEMORY` (RAM-backed transient tables and sorting buffers)
/// - `mmap_size = 256MB` (memory-mapped file I/O for fast access)
///
/// # Errors
/// Returns [`sqlx::Error`] if connection string parsing or pool connection fails.
pub async fn create_sqlite_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .pragma("cache_size", "-64000")
        .pragma("temp_store", "MEMORY")
        .pragma("mmap_size", "268435456");

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}

/// Abstract contract for routine and time-blocking persistence operations.
#[async_trait]
pub trait RoutineRepository: Send + Sync {
    /// Inserts a new routine template for the specified user.
    async fn create_routine(
        &self,
        user_id: Uuid,
        routine: &Routine,
    ) -> Result<(), RoutineRepositoryError>;

    /// Atomically inserts a routine template alongside its initial time blocks within a single transaction for the specified user.
    async fn create_routine_with_blocks(
        &self,
        user_id: Uuid,
        routine: &Routine,
        blocks: &[TimeBlock],
    ) -> Result<(), RoutineRepositoryError>;

    /// Retrieves a routine by its unique identifier and user id.
    async fn get_routine_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError>;

    /// Lists all routines for the specified user.
    async fn list_routines(&self, user_id: Uuid) -> Result<Vec<Routine>, RoutineRepositoryError>;

    /// Retrieves the currently active routine template for the specified user.
    async fn get_active_routine(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError>;

    /// Marks a specific routine as active, deactivating all other routines owned by `user_id`.
    async fn set_active_routine(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError>;

    /// Appends a new time block to a routine owned by `user_id`.
    async fn add_time_block(
        &self,
        block: &TimeBlock,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError>;

    /// Lists all time blocks associated with a routine owned by `user_id`, ordered chronologically.
    async fn list_blocks_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TimeBlock>, RoutineRepositoryError>;

    /// Upserts a daily execution check-in log for a time block on a specific date for a routine owned by `user_id`.
    async fn log_block_status(
        &self,
        block_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
        notes: Option<String>,
    ) -> Result<(), RoutineRepositoryError>;

    /// Retrieves the daily execution log for a specific block and date for a routine owned by `user_id`.
    async fn get_daily_log(
        &self,
        block_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyRoutineLog>, RoutineRepositoryError>;

    /// Retrieves all daily execution logs for a routine on a specific date for a user in a single batch query.
    async fn list_daily_logs_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<DailyRoutineLog>, RoutineRepositoryError>;

    /// Deletes a routine owned by `user_id` and cascades deletion to all associated time blocks and logs.
    async fn delete_routine(&self, id: Uuid, user_id: Uuid) -> Result<(), RoutineRepositoryError>;
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
        user_id: Uuid,
        routine: &Routine,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO routines (id, user_id, name, description, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(routine.id)
        .bind(user_id)
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
    async fn create_routine(
        &self,
        user_id: Uuid,
        routine: &Routine,
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, user_id, routine).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn create_routine_with_blocks(
        &self,
        user_id: Uuid,
        routine: &Routine,
        blocks: &[TimeBlock],
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, user_id, routine).await?;

        for block in blocks {
            Self::insert_block_tx(&mut tx, block).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_routine_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError> {
        let routine = sqlx::query_as::<_, Routine>(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(routine)
    }

    async fn list_routines(&self, user_id: Uuid) -> Result<Vec<Routine>, RoutineRepositoryError> {
        let routines = sqlx::query_as::<_, Routine>(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE user_id = ?
            ORDER BY created_at ASC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(routines)
    }

    async fn get_active_routine(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError> {
        let routine = sqlx::query_as::<_, Routine>(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE is_active = 1 AND user_id = ?
            ORDER BY updated_at DESC
            LIMIT 1
            ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(routine)
    }

    async fn set_active_routine(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // Deactivate all existing routines for this user
        sqlx::query("UPDATE routines SET is_active = 0 WHERE is_active = 1 AND user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Activate selected routine for this user
        let rows_affected = sqlx::query(
            "UPDATE routines SET is_active = 1, updated_at = ? WHERE id = ? AND user_id = ?",
        )
        .bind(chrono::Utc::now())
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(RoutineRepositoryError::NotFound(id));
        }

        tx.commit().await?;
        Ok(())
    }

    async fn add_time_block(
        &self,
        block: &TimeBlock,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError> {
        let _routine = self
            .get_routine_by_id(block.routine_id, user_id)
            .await?
            .ok_or(RoutineRepositoryError::NotFound(block.routine_id))?;

        let mut tx = self.pool.begin().await?;
        Self::insert_block_tx(&mut tx, block).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_blocks_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TimeBlock>, RoutineRepositoryError> {
        let _routine = self
            .get_routine_by_id(routine_id, user_id)
            .await?
            .ok_or(RoutineRepositoryError::NotFound(routine_id))?;

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
        user_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
        notes: Option<String>,
    ) -> Result<(), RoutineRepositoryError> {
        let exists = sqlx::query_scalar::<_, i64>(
            r"
            SELECT 1 FROM time_blocks tb
            JOIN routines r ON tb.routine_id = r.id
            WHERE tb.id = ? AND r.user_id = ?
            ",
        )
        .bind(block_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            return Err(RoutineRepositoryError::BlockNotFound(block_id));
        }

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
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyRoutineLog>, RoutineRepositoryError> {
        let log = sqlx::query_as::<_, DailyRoutineLog>(
            r"
            SELECT dl.id, dl.time_block_id, dl.date, dl.status, dl.started_at, dl.completed_at, dl.notes
            FROM daily_routine_logs dl
            JOIN time_blocks tb ON dl.time_block_id = tb.id
            JOIN routines r ON tb.routine_id = r.id
            WHERE dl.time_block_id = ? AND r.user_id = ? AND dl.date = ?
            ",
        )
        .bind(block_id)
        .bind(user_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(log)
    }

    async fn list_daily_logs_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<DailyRoutineLog>, RoutineRepositoryError> {
        let logs = sqlx::query_as::<_, DailyRoutineLog>(
            r"
            SELECT dl.id, dl.time_block_id, dl.date, dl.status, dl.started_at, dl.completed_at, dl.notes
            FROM daily_routine_logs dl
            JOIN time_blocks tb ON dl.time_block_id = tb.id
            JOIN routines r ON tb.routine_id = r.id
            WHERE r.id = ? AND r.user_id = ? AND dl.date = ?
            ",
        )
        .bind(routine_id)
        .bind(user_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn delete_routine(&self, id: Uuid, user_id: Uuid) -> Result<(), RoutineRepositoryError> {
        let rows_affected = sqlx::query("DELETE FROM routines WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(RoutineRepositoryError::NotFound(id));
        }

        Ok(())
    }
}

/// PostgreSQL-backed implementation of [`RoutineRepository`].
#[derive(Debug, Clone)]
pub struct PgRoutineRepository {
    pool: PgPool,
}

impl PgRoutineRepository {
    /// Creates a new `PgRoutineRepository` wrapping an active PostgreSQL connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying `SQLx` PostgreSQL connection pool.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    fn map_routine_row(row: &sqlx::postgres::PgRow) -> Result<Routine, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let user_id_str: Option<String> = row.try_get("user_id").ok().flatten();
        let user_id = user_id_str.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let is_active: bool = row.try_get("is_active")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        Ok(Routine {
            id,
            user_id,
            name,
            description,
            is_active,
            created_at,
            updated_at,
        })
    }

    fn map_block_row(row: &sqlx::postgres::PgRow) -> Result<TimeBlock, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let routine_id_str: String = row.try_get("routine_id")?;
        let routine_id =
            Uuid::parse_str(&routine_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let name: String = row.try_get("name")?;
        let start_str: String = row.try_get("start_time")?;
        let end_str: String = row.try_get("end_time")?;
        let start_time = NaiveTime::parse_from_str(&start_str, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&start_str, "%H:%M"))
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let end_time = NaiveTime::parse_from_str(&end_str, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(&end_str, "%H:%M"))
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let category: String = row.try_get("category")?;
        let color_hex: String = row.try_get("color_hex")?;
        let order_index: i32 = row.try_get("order_index")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;

        Ok(TimeBlock {
            id,
            routine_id,
            name,
            start_time,
            end_time,
            category,
            color_hex,
            order_index,
            created_at,
        })
    }

    fn map_daily_log_row(row: &sqlx::postgres::PgRow) -> Result<DailyRoutineLog, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let block_id_str: String = row.try_get("time_block_id")?;
        let time_block_id =
            Uuid::parse_str(&block_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let date: NaiveDate = row.try_get("date")?;
        let status_str: String = row.try_get("status")?;
        let status = match status_str.as_str() {
            "in_progress" => BlockStatus::InProgress,
            "completed" => BlockStatus::Completed,
            "skipped" => BlockStatus::Skipped,
            _ => BlockStatus::Pending,
        };
        let started_at: Option<DateTime<Utc>> = row.try_get("started_at")?;
        let completed_at: Option<DateTime<Utc>> = row.try_get("completed_at")?;
        let notes: Option<String> = row.try_get("notes")?;

        Ok(DailyRoutineLog {
            id,
            time_block_id,
            date,
            status,
            started_at,
            completed_at,
            notes,
        })
    }

    async fn insert_routine_tx(
        tx: &mut Transaction<'_, Postgres>,
        user_id: Uuid,
        routine: &Routine,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO routines (id, user_id, name, description, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ",
        )
        .bind(routine.id.to_string())
        .bind(user_id.to_string())
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
        tx: &mut Transaction<'_, Postgres>,
        block: &TimeBlock,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO time_blocks (id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ",
        )
        .bind(block.id.to_string())
        .bind(block.routine_id.to_string())
        .bind(&block.name)
        .bind(block.start_time.format("%H:%M:%S").to_string())
        .bind(block.end_time.format("%H:%M:%S").to_string())
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
impl RoutineRepository for PgRoutineRepository {
    async fn create_routine(
        &self,
        user_id: Uuid,
        routine: &Routine,
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, user_id, routine).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn create_routine_with_blocks(
        &self,
        user_id: Uuid,
        routine: &Routine,
        blocks: &[TimeBlock],
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;
        Self::insert_routine_tx(&mut tx, user_id, routine).await?;

        for block in blocks {
            Self::insert_block_tx(&mut tx, block).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_routine_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE id = $1 AND user_id = $2
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref()
            .map(Self::map_routine_row)
            .transpose()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn list_routines(&self, user_id: Uuid) -> Result<Vec<Routine>, RoutineRepositoryError> {
        let rows = sqlx::query(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE user_id = $1
            ORDER BY created_at ASC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::map_routine_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn get_active_routine(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Routine>, RoutineRepositoryError> {
        let row = sqlx::query(
            r"
            SELECT id, user_id, name, description, is_active, created_at, updated_at
            FROM routines
            WHERE is_active = TRUE AND user_id = $1
            ORDER BY updated_at DESC
            LIMIT 1
            ",
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref()
            .map(Self::map_routine_row)
            .transpose()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn set_active_routine(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE routines SET is_active = FALSE WHERE is_active = TRUE AND user_id = $1",
        )
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

        let rows_affected = sqlx::query(
            "UPDATE routines SET is_active = TRUE, updated_at = $1 WHERE id = $2 AND user_id = $3",
        )
        .bind(Utc::now())
        .bind(id.to_string())
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(RoutineRepositoryError::NotFound(id));
        }

        tx.commit().await?;
        Ok(())
    }

    async fn add_time_block(
        &self,
        block: &TimeBlock,
        user_id: Uuid,
    ) -> Result<(), RoutineRepositoryError> {
        let _routine = self
            .get_routine_by_id(block.routine_id, user_id)
            .await?
            .ok_or(RoutineRepositoryError::NotFound(block.routine_id))?;

        let mut tx = self.pool.begin().await?;
        Self::insert_block_tx(&mut tx, block).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_blocks_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TimeBlock>, RoutineRepositoryError> {
        let _routine = self
            .get_routine_by_id(routine_id, user_id)
            .await?
            .ok_or(RoutineRepositoryError::NotFound(routine_id))?;

        let rows = sqlx::query(
            r"
            SELECT id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at
            FROM time_blocks
            WHERE routine_id = $1
            ORDER BY order_index ASC, start_time ASC
            ",
        )
        .bind(routine_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::map_block_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn log_block_status(
        &self,
        block_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
        notes: Option<String>,
    ) -> Result<(), RoutineRepositoryError> {
        let exists = sqlx::query_scalar::<_, i32>(
            r"
            SELECT 1 FROM time_blocks tb
            JOIN routines r ON tb.routine_id = r.id
            WHERE tb.id = $1 AND r.user_id = $2
            ",
        )
        .bind(block_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            return Err(RoutineRepositoryError::BlockNotFound(block_id));
        }

        let log_id = Uuid::new_v4();
        let now = Utc::now();
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
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT(time_block_id, date) DO UPDATE SET
                status = EXCLUDED.status,
                notes = COALESCE(EXCLUDED.notes, daily_routine_logs.notes),
                started_at = COALESCE(daily_routine_logs.started_at, EXCLUDED.started_at),
                completed_at = COALESCE(EXCLUDED.completed_at, daily_routine_logs.completed_at)
            ",
        )
        .bind(log_id.to_string())
        .bind(block_id.to_string())
        .bind(date)
        .bind(status.as_str())
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
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<DailyRoutineLog>, RoutineRepositoryError> {
        let row = sqlx::query(
            r"
            SELECT dl.id, dl.time_block_id, dl.date, dl.status, dl.started_at, dl.completed_at, dl.notes
            FROM daily_routine_logs dl
            JOIN time_blocks tb ON dl.time_block_id = tb.id
            JOIN routines r ON tb.routine_id = r.id
            WHERE dl.time_block_id = $1 AND r.user_id = $2 AND dl.date = $3
            ",
        )
        .bind(block_id.to_string())
        .bind(user_id.to_string())
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref()
            .map(Self::map_daily_log_row)
            .transpose()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn list_daily_logs_for_routine(
        &self,
        routine_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<DailyRoutineLog>, RoutineRepositoryError> {
        let rows = sqlx::query(
            r"
            SELECT dl.id, dl.time_block_id, dl.date, dl.status, dl.started_at, dl.completed_at, dl.notes
            FROM daily_routine_logs dl
            JOIN time_blocks tb ON dl.time_block_id = tb.id
            JOIN routines r ON tb.routine_id = r.id
            WHERE r.id = $1 AND r.user_id = $2 AND dl.date = $3
            ",
        )
        .bind(routine_id.to_string())
        .bind(user_id.to_string())
        .bind(date)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(Self::map_daily_log_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(RoutineRepositoryError::Database)
    }

    async fn delete_routine(&self, id: Uuid, user_id: Uuid) -> Result<(), RoutineRepositoryError> {
        let rows_affected = sqlx::query("DELETE FROM routines WHERE id = $1 AND user_id = $2")
            .bind(id.to_string())
            .bind(user_id.to_string())
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
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let routine = Routine::new("Morning Focus", Some("High performance morning".into()))
            .with_user_id(user_id);
        repo.create_routine(user_id, &routine)
            .await
            .expect("create routine should succeed");

        let fetched = repo
            .get_routine_by_id(routine.id, user_id)
            .await
            .expect("get routine should succeed")
            .expect("routine must be present");

        assert_eq!(fetched.name, "Morning Focus");
        assert_eq!(fetched.user_id, Some(user_id));
        assert!(fetched.is_active);

        // Active routine query
        let active = repo
            .get_active_routine(user_id)
            .await
            .expect("get active routine should succeed")
            .expect("active routine must exist");
        assert_eq!(active.id, routine.id);

        let list = repo.list_routines(user_id).await.expect("list routines");
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_routine_multi_tenant_isolation() {
        let repo = setup_test_repo().await;
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_a)
            .await
            .unwrap();
        crate::domain::seed_test_user(repo.pool(), user_b)
            .await
            .unwrap();

        let routine_a = Routine::new("User A Routine", None).with_user_id(user_a);
        let block_a = TimeBlock::new(
            routine_a.id,
            "Block A",
            NaiveTime::from_hms_opt(9, 0, 0).expect("time"),
            NaiveTime::from_hms_opt(10, 0, 0).expect("time"),
            1,
        );
        repo.create_routine_with_blocks(user_a, &routine_a, std::slice::from_ref(&block_a))
            .await
            .expect("create routine a");

        // User B cannot see routine A
        assert!(repo
            .get_routine_by_id(routine_a.id, user_b)
            .await
            .expect("get")
            .is_none());

        // User B's routines list is empty
        let b_routines = repo.list_routines(user_b).await.expect("list");
        assert_eq!(b_routines.len(), 0);

        // User B cannot fetch active routine of A
        assert!(repo
            .get_active_routine(user_b)
            .await
            .expect("active")
            .is_none());

        // User B cannot list blocks of routine A
        let list_res = repo.list_blocks_for_routine(routine_a.id, user_b).await;
        assert!(matches!(list_res, Err(RoutineRepositoryError::NotFound(_))));

        // User B cannot log block status for block A
        let date = NaiveDate::from_ymd_opt(2026, 11, 1).expect("date");
        let log_res = repo
            .log_block_status(block_a.id, user_b, date, BlockStatus::Completed, None)
            .await;
        assert!(matches!(
            log_res,
            Err(RoutineRepositoryError::BlockNotFound(_))
        ));

        // User B cannot delete routine A
        let del_res = repo.delete_routine(routine_a.id, user_b).await;
        assert!(matches!(del_res, Err(RoutineRepositoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_sqlite_transactional_create_routine_with_blocks() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let routine = Routine::new("Full Day Flow", None).with_user_id(user_id);
        let start1 = NaiveTime::from_hms_opt(8, 0, 0).expect("valid time");
        let end1 = NaiveTime::from_hms_opt(9, 30, 0).expect("valid time");
        let start2 = NaiveTime::from_hms_opt(10, 0, 0).expect("valid time");
        let end2 = NaiveTime::from_hms_opt(12, 0, 0).expect("valid time");

        let blocks = vec![
            TimeBlock::new(routine.id, "Morning Reading", start1, end1, 1),
            TimeBlock::new(routine.id, "Deep Work", start2, end2, 2),
        ];

        repo.create_routine_with_blocks(user_id, &routine, &blocks)
            .await
            .expect("transactional creation should succeed");

        let fetched_blocks = repo
            .list_blocks_for_routine(routine.id, user_id)
            .await
            .expect("list blocks should succeed");

        assert_eq!(fetched_blocks.len(), 2);
        assert_eq!(fetched_blocks[0].name, "Morning Reading");
        assert_eq!(fetched_blocks[1].name, "Deep Work");
    }

    #[tokio::test]
    async fn test_sqlite_log_block_status_upsert_idempotence() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let routine = Routine::new("Evening Winddown", None).with_user_id(user_id);
        let start = NaiveTime::from_hms_opt(21, 0, 0).expect("valid time");
        let end = NaiveTime::from_hms_opt(22, 0, 0).expect("valid time");
        let block = TimeBlock::new(routine.id, "Meditation", start, end, 1);

        repo.create_routine(user_id, &routine)
            .await
            .expect("routine insert");
        repo.add_time_block(&block, user_id)
            .await
            .expect("block insert");

        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");

        // 1. Log In Progress
        repo.log_block_status(
            block.id,
            user_id,
            date,
            BlockStatus::InProgress,
            Some("Starting session".into()),
        )
        .await
        .expect("log in progress");

        let log = repo
            .get_daily_log(block.id, user_id, date)
            .await
            .expect("get log")
            .expect("log exists");
        assert_eq!(log.status, BlockStatus::InProgress);
        assert_eq!(log.notes.as_deref(), Some("Starting session"));
        assert!(log.started_at.is_some());

        // 2. Upsert to Completed (same block and date)
        repo.log_block_status(
            block.id,
            user_id,
            date,
            BlockStatus::Completed,
            Some("Finished peacefully".into()),
        )
        .await
        .expect("log completed");

        let updated_log = repo
            .get_daily_log(block.id, user_id, date)
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
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let routine = Routine::new("Temporary Routine", None).with_user_id(user_id);
        let start = NaiveTime::from_hms_opt(14, 0, 0).expect("valid time");
        let end = NaiveTime::from_hms_opt(15, 0, 0).expect("valid time");
        let block = TimeBlock::new(routine.id, "Quick Sync", start, end, 1);

        repo.create_routine_with_blocks(user_id, &routine, &[block])
            .await
            .expect("insert");

        let blocks_before = repo
            .list_blocks_for_routine(routine.id, user_id)
            .await
            .expect("list");
        assert_eq!(blocks_before.len(), 1);

        // Delete parent routine
        repo.delete_routine(routine.id, user_id)
            .await
            .expect("delete");

        let list_res = repo.list_blocks_for_routine(routine.id, user_id).await;
        assert!(matches!(list_res, Err(RoutineRepositoryError::NotFound(_))));

        let routine_opt = repo
            .get_routine_by_id(routine.id, user_id)
            .await
            .expect("get");
        assert!(routine_opt.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_list_daily_logs_for_routine_batch() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let routine = Routine::new("Batch Log Routine", None).with_user_id(user_id);
        let block1 = TimeBlock::new(
            routine.id,
            "Block 1",
            NaiveTime::from_hms_opt(8, 0, 0).expect("valid time"),
            NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"),
            1,
        );
        let block2 = TimeBlock::new(
            routine.id,
            "Block 2",
            NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"),
            NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"),
            2,
        );

        repo.create_routine_with_blocks(user_id, &routine, &[block1.clone(), block2.clone()])
            .await
            .expect("insert routine with blocks");

        let today = chrono::Utc::now().date_naive();
        repo.log_block_status(block1.id, user_id, today, BlockStatus::Completed, None)
            .await
            .expect("log block 1");
        repo.log_block_status(block2.id, user_id, today, BlockStatus::InProgress, None)
            .await
            .expect("log block 2");

        let logs = repo
            .list_daily_logs_for_routine(routine.id, user_id, today)
            .await
            .expect("list batch logs");

        assert_eq!(logs.len(), 2);
    }
}
