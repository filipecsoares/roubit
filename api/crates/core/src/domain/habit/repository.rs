//! Habit SQLite Repository with Atomic Check-Ins (RF02)
//!
//! Provides asynchronous, thread-safe persistence for habits and daily habit logs,
//! featuring atomic UPSERT check-ins with target completion evaluation.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::models::{Habit, HabitLog};

/// Errors occurring during habit repository operations.
#[derive(Debug, thiserror::Error)]
pub enum HabitRepositoryError {
    /// Internal `SQLx` database error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Target habit entity was not found.
    #[error("Habit not found: {0}")]
    NotFound(Uuid),
}

/// Abstract contract for habit persistence operations.
#[async_trait]
pub trait HabitRepository: Send + Sync {
    /// Inserts a new habit definition.
    async fn create_habit(&self, habit: &Habit) -> Result<(), HabitRepositoryError>;

    /// Retrieves a habit by its unique identifier.
    async fn get_habit_by_id(&self, id: Uuid) -> Result<Option<Habit>, HabitRepositoryError>;

    /// Lists all active (non-archived) habits.
    async fn list_active_habits(&self) -> Result<Vec<Habit>, HabitRepositoryError>;

    /// Atomically records a daily check-in for a habit.
    ///
    /// If `is_increment` is `true`, `value` is added to the existing `actual_value` for that date.
    /// If `is_increment` is `false`, `value` replaces the existing `actual_value`.
    /// `is_completed` is automatically evaluated against the habit's `target_value`.
    async fn record_check_in(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
        value: f64,
        is_increment: bool,
    ) -> Result<HabitLog, HabitRepositoryError>;

    /// Retrieves chronological habit logs within a bounded date range `[start_date, end_date]`.
    async fn get_logs_for_habit(
        &self,
        habit_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<HabitLog>, HabitRepositoryError>;

    /// Marks a habit as archived.
    async fn archive_habit(&self, id: Uuid) -> Result<(), HabitRepositoryError>;

    /// Permanently deletes a habit and cascades deletion to all associated logs.
    async fn delete_habit(&self, id: Uuid) -> Result<(), HabitRepositoryError>;
}

/// SQLite-backed implementation of [`HabitRepository`].
#[derive(Debug, Clone)]
pub struct SqliteHabitRepository {
    pool: SqlitePool,
}

impl SqliteHabitRepository {
    /// Creates a new `SqliteHabitRepository` wrapping an active connection pool.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying `SQLx` SQLite connection pool.
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl HabitRepository for SqliteHabitRepository {
    async fn create_habit(&self, habit: &Habit) -> Result<(), HabitRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO habits (id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(habit.id)
        .bind(&habit.name)
        .bind(&habit.description)
        .bind(habit.habit_type)
        .bind(habit.target_value)
        .bind(&habit.unit)
        .bind(habit.frequency_days)
        .bind(&habit.color_hex)
        .bind(habit.is_archived)
        .bind(habit.created_at)
        .bind(habit.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_habit_by_id(&self, id: Uuid) -> Result<Option<Habit>, HabitRepositoryError> {
        let habit = sqlx::query_as::<_, Habit>(
            r"
            SELECT id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at
            FROM habits
            WHERE id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(habit)
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, HabitRepositoryError> {
        let habits = sqlx::query_as::<_, Habit>(
            r"
            SELECT id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at
            FROM habits
            WHERE is_archived = 0
            ORDER BY created_at ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(habits)
    }

    async fn record_check_in(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
        value: f64,
        is_increment: bool,
    ) -> Result<HabitLog, HabitRepositoryError> {
        let habit = self
            .get_habit_by_id(habit_id)
            .await?
            .ok_or(HabitRepositoryError::NotFound(habit_id))?;

        let log_id = Uuid::new_v4();
        let now: DateTime<Utc> = Utc::now();
        let target_value = habit.target_value;
        let initial_completed = value >= target_value;

        let log: HabitLog = if is_increment {
            sqlx::query_as::<_, HabitLog>(
                r"
                INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
                VALUES (?, ?, ?, ?, ?, NULL, ?)
                ON CONFLICT(habit_id, log_date) DO UPDATE SET
                    actual_value = habit_logs.actual_value + excluded.actual_value,
                    is_completed = (habit_logs.actual_value + excluded.actual_value >= ?),
                    logged_at = excluded.logged_at
                RETURNING id, habit_id, log_date, actual_value, is_completed, notes, logged_at
                ",
            )
            .bind(log_id)
            .bind(habit_id)
            .bind(date)
            .bind(value)
            .bind(initial_completed)
            .bind(now)
            .bind(target_value)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, HabitLog>(
                r"
                INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
                VALUES (?, ?, ?, ?, ?, NULL, ?)
                ON CONFLICT(habit_id, log_date) DO UPDATE SET
                    actual_value = excluded.actual_value,
                    is_completed = (excluded.actual_value >= ?),
                    logged_at = excluded.logged_at
                RETURNING id, habit_id, log_date, actual_value, is_completed, notes, logged_at
                ",
            )
            .bind(log_id)
            .bind(habit_id)
            .bind(date)
            .bind(value)
            .bind(initial_completed)
            .bind(now)
            .bind(target_value)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(log)
    }

    async fn get_logs_for_habit(
        &self,
        habit_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<HabitLog>, HabitRepositoryError> {
        let logs = sqlx::query_as::<_, HabitLog>(
            r"
            SELECT id, habit_id, log_date, actual_value, is_completed, notes, logged_at
            FROM habit_logs
            WHERE habit_id = ? AND log_date >= ? AND log_date <= ?
            ORDER BY log_date ASC
            ",
        )
        .bind(habit_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn archive_habit(&self, id: Uuid) -> Result<(), HabitRepositoryError> {
        let rows_affected = sqlx::query(
            r"
            UPDATE habits
            SET is_archived = 1, updated_at = ?
            WHERE id = ?
            ",
        )
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(HabitRepositoryError::NotFound(id));
        }

        Ok(())
    }

    async fn delete_habit(&self, id: Uuid) -> Result<(), HabitRepositoryError> {
        let rows_affected = sqlx::query("DELETE FROM habits WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(HabitRepositoryError::NotFound(id));
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::similar_names,
    clippy::assert_is_empty
)]
mod tests {
    use super::*;
    use crate::domain::create_sqlite_pool;

    async fn setup_repo() -> SqliteHabitRepository {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("in-memory pool succeeds");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations succeed");

        SqliteHabitRepository::new(pool)
    }

    #[tokio::test]
    async fn test_habit_crud_and_archival() {
        let repo = setup_repo().await;

        let habit = Habit::new_numeric("Water Intake", None, 2000.0, "ml");
        repo.create_habit(&habit).await.expect("create succeeds");

        let fetched = repo
            .get_habit_by_id(habit.id)
            .await
            .expect("get succeeds")
            .expect("habit exists");
        assert_eq!(fetched.id, habit.id);
        assert_eq!(fetched.name, "Water Intake");
        assert_eq!(fetched.target_value, 2000.0);
        assert_eq!(fetched.unit.as_deref(), Some("ml"));
        assert!(!fetched.is_archived);

        let active_habits = repo.list_active_habits().await.expect("list succeeds");
        assert_eq!(active_habits.len(), 1);

        repo.archive_habit(habit.id)
            .await
            .expect("archive succeeds");
        let active_after = repo.list_active_habits().await.expect("list succeeds");
        assert_eq!(active_after.len(), 0);

        let archived_habit = repo
            .get_habit_by_id(habit.id)
            .await
            .expect("get succeeds")
            .expect("habit exists");
        assert!(archived_habit.is_archived);

        repo.delete_habit(habit.id).await.expect("delete succeeds");
        let deleted = repo.get_habit_by_id(habit.id).await.expect("get succeeds");
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_habit_checkin_upsert_and_increment() {
        let repo = setup_repo().await;

        let habit = Habit::new_numeric("Drink Water", None, 2000.0, "ml");
        repo.create_habit(&habit).await.expect("create succeeds");

        let today = Utc::now().date_naive();

        // 1. First drink: 500ml (actual: 500.0 < 2000.0 -> not completed)
        let log1 = repo
            .record_check_in(habit.id, today, 500.0, true)
            .await
            .expect("first check-in succeeds");
        assert_eq!(log1.actual_value, 500.0);
        assert!(!log1.is_completed);

        // 2. Second drink: 1600ml (increment: 500.0 + 1600.0 = 2100.0 >= 2000.0 -> completed)
        let log2 = repo
            .record_check_in(habit.id, today, 1600.0, true)
            .await
            .expect("second check-in succeeds");
        assert_eq!(log2.actual_value, 2100.0);
        assert!(log2.is_completed);

        // 3. Overwrite check-in (is_increment = false): reset to 800ml -> not completed
        let log3 = repo
            .record_check_in(habit.id, today, 800.0, false)
            .await
            .expect("overwrite check-in succeeds");
        assert_eq!(log3.actual_value, 800.0);
        assert!(!log3.is_completed);

        // 4. Verify exactly 1 log record exists for (habit.id, today)
        let logs = repo
            .get_logs_for_habit(habit.id, today, today)
            .await
            .expect("get logs succeeds");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].actual_value, 800.0);
    }

    #[tokio::test]
    async fn test_record_check_in_not_found() {
        let repo = setup_repo().await;
        let non_existent_id = Uuid::new_v4();
        let today = Utc::now().date_naive();

        let result = repo
            .record_check_in(non_existent_id, today, 1.0, false)
            .await;
        assert!(matches!(result, Err(HabitRepositoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_logs_range_ordering() {
        let repo = setup_repo().await;
        let habit = Habit::new_binary("Floss", None);
        repo.create_habit(&habit).await.expect("create succeeds");

        let d1 = NaiveDate::from_ymd_opt(2026, 10, 20).expect("valid date");
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 21).expect("valid date");
        let d3 = NaiveDate::from_ymd_opt(2026, 10, 22).expect("valid date");

        // Insert in reverse order
        repo.record_check_in(habit.id, d3, 1.0, false)
            .await
            .expect("d3");
        repo.record_check_in(habit.id, d1, 1.0, false)
            .await
            .expect("d1");
        repo.record_check_in(habit.id, d2, 1.0, false)
            .await
            .expect("d2");

        let logs = repo
            .get_logs_for_habit(habit.id, d1, d3)
            .await
            .expect("fetch logs succeeds");

        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].log_date, d1);
        assert_eq!(logs[1].log_date, d2);
        assert_eq!(logs[2].log_date, d3);
    }
}
