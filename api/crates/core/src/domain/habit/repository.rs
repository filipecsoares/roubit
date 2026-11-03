//! Habit SQLite Repository with Atomic Check-Ins (RF02)
//!
//! Provides asynchronous, thread-safe persistence for habits and daily habit logs,
//! featuring atomic UPSERT check-ins with target completion evaluation.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::{
    models::{Habit, HabitLog},
    streak::{StreakCalculator, StreakMetrics},
};

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
    /// Inserts a new habit definition for the specified user.
    async fn create_habit(&self, user_id: Uuid, habit: &Habit) -> Result<(), HabitRepositoryError>;

    /// Retrieves a habit by its unique identifier and user id.
    async fn get_habit_by_id(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Habit>, HabitRepositoryError>;

    /// Lists all active (non-archived) habits for the specified user.
    async fn list_active_habits(&self, user_id: Uuid) -> Result<Vec<Habit>, HabitRepositoryError>;

    /// Lists all habits, including archived ones, for the specified user.
    async fn list_all_habits(&self, user_id: Uuid) -> Result<Vec<Habit>, HabitRepositoryError>;

    /// Atomically records a daily check-in for a habit owned by `user_id`.
    ///
    /// If `is_increment` is `true`, `value` is added to the existing `actual_value` for that date.
    /// If `is_increment` is `false`, `value` replaces the existing `actual_value`.
    /// `is_completed` is automatically evaluated against the habit's `target_value`.
    async fn record_check_in(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
        value: f64,
        is_increment: bool,
    ) -> Result<HabitLog, HabitRepositoryError>;

    /// Retrieves chronological habit logs within a bounded date range `[start_date, end_date]` for a habit owned by `user_id`.
    async fn get_logs_for_habit(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<HabitLog>, HabitRepositoryError>;

    /// Marks a habit as archived.
    async fn archive_habit(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), HabitRepositoryError>;

    /// Permanently deletes a habit and cascades deletion to all associated logs.
    async fn delete_habit(&self, habit_id: Uuid, user_id: Uuid)
        -> Result<(), HabitRepositoryError>;

    /// Retrieves a set of habit IDs completed by `user_id` on the specified date.
    async fn get_completed_habit_ids_for_date(
        &self,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<HashSet<Uuid>, HabitRepositoryError>;

    /// Checks if a specific habit has a completed log on the specified date.
    async fn is_habit_completed_on_date(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
    ) -> Result<bool, HabitRepositoryError>;

    /// Lists active habits for `user_id` alongside their calculated streak metrics evaluated for `reference_date`.
    async fn list_active_habits_with_streaks(
        &self,
        user_id: Uuid,
        reference_date: NaiveDate,
    ) -> Result<Vec<(Habit, StreakMetrics)>, HabitRepositoryError>;
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
#[allow(clippy::too_many_lines)]
impl HabitRepository for SqliteHabitRepository {
    async fn create_habit(&self, user_id: Uuid, habit: &Habit) -> Result<(), HabitRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO habits (id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(habit.id)
        .bind(user_id)
        .bind(&habit.name)
        .bind(&habit.description)
        .bind(habit.habit_type)
        .bind(habit.target_value)
        .bind(&habit.unit)
        .bind(habit.frequency_days)
        .bind(&habit.color_hex)
        .bind(habit.is_archived)
        .bind(habit.current_streak)
        .bind(habit.longest_streak)
        .bind(habit.created_at)
        .bind(habit.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_habit_by_id(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Habit>, HabitRepositoryError> {
        let habit = sqlx::query_as::<_, Habit>(
            r"
            SELECT id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at
            FROM habits
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(habit_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(habit)
    }

    async fn list_active_habits(&self, user_id: Uuid) -> Result<Vec<Habit>, HabitRepositoryError> {
        let habits = sqlx::query_as::<_, Habit>(
            r"
            SELECT id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at
            FROM habits
            WHERE is_archived = 0 AND user_id = ?
            ORDER BY created_at ASC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(habits)
    }

    async fn list_all_habits(&self, user_id: Uuid) -> Result<Vec<Habit>, HabitRepositoryError> {
        let habits = sqlx::query_as::<_, Habit>(
            r"
            SELECT id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at
            FROM habits
            WHERE user_id = ?
            ORDER BY created_at ASC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(habits)
    }

    async fn record_check_in(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
        date: NaiveDate,
        value: f64,
        is_increment: bool,
    ) -> Result<HabitLog, HabitRepositoryError> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

        let res: Result<HabitLog, HabitRepositoryError> = async {
            let habit = sqlx::query_as::<_, Habit>(
                r"
                SELECT id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at
                FROM habits
                WHERE id = ? AND user_id = ?
                ",
            )
            .bind(habit_id)
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(HabitRepositoryError::NotFound(habit_id))?;

            // Check if there was an existing completed log for this date to avoid double-incrementing streak
            let existing_log: Option<HabitLog> = sqlx::query_as(
                r"
                SELECT id, habit_id, log_date, actual_value, is_completed, notes, logged_at
                FROM habit_logs
                WHERE habit_id = ? AND log_date = ?
                ",
            )
            .bind(habit_id)
            .bind(date)
            .fetch_optional(&mut *conn)
            .await?;

            let was_already_completed = existing_log.as_ref().is_some_and(|l| l.is_completed);

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
                .fetch_one(&mut *conn)
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
                .fetch_one(&mut *conn)
                .await?
            };

            // Atomically update habit's denormalized streak when habit reaches completed state
            if log.is_completed && !was_already_completed {
                let last_completed_date: Option<NaiveDate> = sqlx::query_scalar(
                    r"
                    SELECT log_date
                    FROM habit_logs
                    WHERE habit_id = ? AND log_date < ? AND is_completed = 1
                    ORDER BY log_date DESC
                    LIMIT 1
                    ",
                )
                .bind(habit_id)
                .bind(date)
                .fetch_optional(&mut *conn)
                .await?;

                let (new_current_streak, new_longest_streak) =
                    StreakCalculator::update_streak_incremental(
                        habit.current_streak,
                        habit.longest_streak,
                        last_completed_date,
                        date,
                        habit.frequency_days,
                    );

                sqlx::query(
                    r"
                    UPDATE habits
                    SET current_streak = ?, longest_streak = ?, updated_at = ?
                    WHERE id = ? AND user_id = ?
                    ",
                )
                .bind(new_current_streak)
                .bind(new_longest_streak)
                .bind(now)
                .bind(habit_id)
                .bind(user_id)
                .execute(&mut *conn)
                .await?;
            }

            Ok(log)
        }
        .await;

        match res {
            Ok(log) => {
                sqlx::query("COMMIT").execute(&mut *conn).await?;
                Ok(log)
            }
            Err(e) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                Err(e)
            }
        }
    }

    async fn get_logs_for_habit(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<HabitLog>, HabitRepositoryError> {
        let _habit = self
            .get_habit_by_id(habit_id, user_id)
            .await?
            .ok_or(HabitRepositoryError::NotFound(habit_id))?;

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

    async fn archive_habit(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), HabitRepositoryError> {
        let rows_affected = sqlx::query(
            r"
            UPDATE habits
            SET is_archived = 1, updated_at = ?
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(Utc::now())
        .bind(habit_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(HabitRepositoryError::NotFound(habit_id));
        }

        Ok(())
    }

    async fn delete_habit(
        &self,
        habit_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), HabitRepositoryError> {
        let rows_affected = sqlx::query("DELETE FROM habits WHERE id = ? AND user_id = ?")
            .bind(habit_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(HabitRepositoryError::NotFound(habit_id));
        }

        Ok(())
    }

    async fn get_completed_habit_ids_for_date(
        &self,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<HashSet<Uuid>, HabitRepositoryError> {
        let habit_ids: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
            r"
            SELECT hl.habit_id
            FROM habit_logs hl
            JOIN habits h ON hl.habit_id = h.id
            WHERE h.user_id = ? AND hl.log_date = ? AND hl.is_completed = 1
            ",
        )
        .bind(user_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await?;

        Ok(habit_ids.into_iter().collect())
    }

    async fn is_habit_completed_on_date(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
    ) -> Result<bool, HabitRepositoryError> {
        let is_completed = sqlx::query_scalar::<_, bool>(
            r"SELECT is_completed FROM habit_logs WHERE habit_id = ? AND log_date = ?",
        )
        .bind(habit_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(false);

        Ok(is_completed)
    }

    async fn list_active_habits_with_streaks(
        &self,
        user_id: Uuid,
        reference_date: NaiveDate,
    ) -> Result<Vec<(Habit, StreakMetrics)>, HabitRepositoryError> {
        let habits = self.list_active_habits(user_id).await?;
        let completed_ids = self
            .get_completed_habit_ids_for_date(user_id, reference_date)
            .await?;

        let mut result = Vec::with_capacity(habits.len());
        for habit in habits {
            let is_active_today = completed_ids.contains(&habit.id);
            let streak = StreakMetrics {
                current_streak: u32::try_from(habit.current_streak.max(0)).unwrap_or(0),
                longest_streak: u32::try_from(habit.longest_streak.max(0)).unwrap_or(0),
                is_active_today,
            };
            result.push((habit, streak));
        }

        Ok(result)
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
    use crate::domain::{create_sqlite_pool, seed_test_user};

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
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();

        let habit = Habit::new_numeric("Water Intake", None, 2000.0, "ml").with_user_id(user_id);
        repo.create_habit(user_id, &habit)
            .await
            .expect("create succeeds");

        let fetched = repo
            .get_habit_by_id(habit.id, user_id)
            .await
            .expect("get succeeds")
            .expect("habit exists");
        assert_eq!(fetched.id, habit.id);
        assert_eq!(fetched.user_id, Some(user_id));
        assert_eq!(fetched.name, "Water Intake");
        assert_eq!(fetched.target_value, 2000.0);
        assert_eq!(fetched.unit.as_deref(), Some("ml"));
        assert!(!fetched.is_archived);

        let active_habits = repo
            .list_active_habits(user_id)
            .await
            .expect("list succeeds");
        assert_eq!(active_habits.len(), 1);

        repo.archive_habit(habit.id, user_id)
            .await
            .expect("archive succeeds");
        let active_after = repo
            .list_active_habits(user_id)
            .await
            .expect("list succeeds");
        assert_eq!(active_after.len(), 0);

        let archived_habit = repo
            .get_habit_by_id(habit.id, user_id)
            .await
            .expect("get succeeds")
            .expect("habit exists");
        assert!(archived_habit.is_archived);

        repo.delete_habit(habit.id, user_id)
            .await
            .expect("delete succeeds");
        let deleted = repo
            .get_habit_by_id(habit.id, user_id)
            .await
            .expect("get succeeds");
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_habit_multi_tenant_isolation() {
        let repo = setup_repo().await;
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        seed_test_user(repo.pool(), user_a).await.unwrap();
        seed_test_user(repo.pool(), user_b).await.unwrap();

        let habit_a = Habit::new_numeric("Habit A", None, 100.0, "count").with_user_id(user_a);
        repo.create_habit(user_a, &habit_a)
            .await
            .expect("create habit a");

        // User B cannot see user A's habit
        assert!(repo
            .get_habit_by_id(habit_a.id, user_b)
            .await
            .expect("get")
            .is_none());

        // User B's active list is empty
        let b_list = repo.list_active_habits(user_b).await.expect("list active");
        assert!(b_list.is_empty());

        // User B cannot check-in to user A's habit
        let today = Utc::now().date_naive();
        let checkin_res = repo
            .record_check_in(habit_a.id, user_b, today, 50.0, true)
            .await;
        assert!(matches!(
            checkin_res,
            Err(HabitRepositoryError::NotFound(_))
        ));

        // User B cannot archive user A's habit
        let archive_res = repo.archive_habit(habit_a.id, user_b).await;
        assert!(matches!(
            archive_res,
            Err(HabitRepositoryError::NotFound(_))
        ));

        // User B cannot delete user A's habit
        let delete_res = repo.delete_habit(habit_a.id, user_b).await;
        assert!(matches!(delete_res, Err(HabitRepositoryError::NotFound(_))));

        // Habit A is still intact and accessible for User A
        let fetched_a = repo
            .get_habit_by_id(habit_a.id, user_a)
            .await
            .expect("get a")
            .expect("exists");
        assert_eq!(fetched_a.id, habit_a.id);
    }

    #[tokio::test]
    async fn test_habit_checkin_upsert_and_increment() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();

        let habit = Habit::new_numeric("Drink Water", None, 2000.0, "ml").with_user_id(user_id);
        repo.create_habit(user_id, &habit)
            .await
            .expect("create succeeds");

        let today = Utc::now().date_naive();

        // 1. First drink: 500ml (actual: 500.0 < 2000.0 -> not completed)
        let log1 = repo
            .record_check_in(habit.id, user_id, today, 500.0, true)
            .await
            .expect("first check-in succeeds");
        assert_eq!(log1.actual_value, 500.0);
        assert!(!log1.is_completed);

        // 2. Second drink: 1600ml (increment: 500.0 + 1600.0 = 2100.0 >= 2000.0 -> completed)
        let log2 = repo
            .record_check_in(habit.id, user_id, today, 1600.0, true)
            .await
            .expect("second check-in succeeds");
        assert_eq!(log2.actual_value, 2100.0);
        assert!(log2.is_completed);

        // 3. Overwrite check-in (is_increment = false): reset to 800ml -> not completed
        let log3 = repo
            .record_check_in(habit.id, user_id, today, 800.0, false)
            .await
            .expect("overwrite check-in succeeds");
        assert_eq!(log3.actual_value, 800.0);
        assert!(!log3.is_completed);

        // 4. Verify exactly 1 log record exists for (habit.id, today)
        let logs = repo
            .get_logs_for_habit(habit.id, user_id, today, today)
            .await
            .expect("get logs succeeds");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].actual_value, 800.0);
    }

    #[tokio::test]
    async fn test_record_check_in_not_found() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();
        let non_existent_id = Uuid::new_v4();
        let today = Utc::now().date_naive();

        let result = repo
            .record_check_in(non_existent_id, user_id, today, 1.0, false)
            .await;
        assert!(matches!(result, Err(HabitRepositoryError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_logs_range_ordering() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();
        let habit = Habit::new_binary("Floss", None).with_user_id(user_id);
        repo.create_habit(user_id, &habit)
            .await
            .expect("create succeeds");

        let d1 = NaiveDate::from_ymd_opt(2026, 10, 20).expect("valid date");
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 21).expect("valid date");
        let d3 = NaiveDate::from_ymd_opt(2026, 10, 22).expect("valid date");

        // Insert in reverse order
        repo.record_check_in(habit.id, user_id, d3, 1.0, false)
            .await
            .expect("d3");
        repo.record_check_in(habit.id, user_id, d1, 1.0, false)
            .await
            .expect("d1");
        repo.record_check_in(habit.id, user_id, d2, 1.0, false)
            .await
            .expect("d2");

        let logs = repo
            .get_logs_for_habit(habit.id, user_id, d1, d3)
            .await
            .expect("fetch logs succeeds");

        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].log_date, d1);
        assert_eq!(logs[1].log_date, d2);
        assert_eq!(logs[2].log_date, d3);
    }

    #[tokio::test]
    async fn test_habit_denormalized_streak_columns_persisted() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();
        let mut habit = Habit::new_binary("Denorm Habit", None).with_user_id(user_id);
        habit.current_streak = 5;
        habit.longest_streak = 12;

        repo.create_habit(user_id, &habit)
            .await
            .expect("create succeeds");

        let fetched = repo
            .get_habit_by_id(habit.id, user_id)
            .await
            .expect("get succeeds")
            .expect("habit found");

        assert_eq!(fetched.current_streak, 5);
        assert_eq!(fetched.longest_streak, 12);

        let list = repo
            .list_active_habits(user_id)
            .await
            .expect("list succeeds");
        assert_eq!(list[0].current_streak, 5);
        assert_eq!(list[0].longest_streak, 12);
    }

    #[tokio::test]
    async fn test_habit_active_streaks_and_date_queries() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        seed_test_user(repo.pool(), user_id).await.unwrap();

        let habit1 = Habit::new_binary("Habit One", None).with_user_id(user_id);
        let habit2 = Habit::new_binary("Habit Two", None).with_user_id(user_id);
        repo.create_habit(user_id, &habit1).await.unwrap();
        repo.create_habit(user_id, &habit2).await.unwrap();

        let today = Utc::now().date_naive();
        // Check-in habit1 today
        repo.record_check_in(habit1.id, user_id, today, 1.0, false)
            .await
            .unwrap();

        // Test get_completed_habit_ids_for_date
        let completed_ids = repo
            .get_completed_habit_ids_for_date(user_id, today)
            .await
            .unwrap();
        assert!(completed_ids.contains(&habit1.id));
        assert!(!completed_ids.contains(&habit2.id));

        // Test is_habit_completed_on_date
        assert!(repo.is_habit_completed_on_date(habit1.id, today).await.unwrap());
        assert!(!repo.is_habit_completed_on_date(habit2.id, today).await.unwrap());

        // Test list_active_habits_with_streaks
        let active_with_streaks = repo
            .list_active_habits_with_streaks(user_id, today)
            .await
            .unwrap();
        assert_eq!(active_with_streaks.len(), 2);
        let h1_res = active_with_streaks
            .iter()
            .find(|(h, _)| h.id == habit1.id)
            .unwrap();
        assert!(h1_res.1.is_active_today);
        let h2_res = active_with_streaks
            .iter()
            .find(|(h, _)| h.id == habit2.id)
            .unwrap();
        assert!(!h2_res.1.is_active_today);
    }
}
