//! Local client adapter delegating directly to SQLite repositories.

use async_trait::async_trait;
use chrono::NaiveDate;
use roubit_core::{
    domain::{
        habit::{
            models::{Habit, HabitLog},
            HabitRepository, SqliteHabitRepository,
        },
        journal::{models::JournalMetadata, JournalRepository, SqliteJournalRepository},
        okr::{OkrRepository, SqliteOkrRepository},
        routine::{
            models::{Routine, TimeBlock},
            RoutineRepository, SqliteRoutineRepository,
        },
        BlockStatus, ObjectiveWithKeyResults,
    },
    LOCAL_USER_ID,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::{ClientError, RoubitClient};

/// Client implementation connecting directly to an embedded SQLite database.
#[derive(Debug, Clone)]
pub struct LocalClient {
    pool: SqlitePool,
}

impl LocalClient {
    /// Creates a new `LocalClient` instance wrapping an active SQLite connection pool.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying SQLite pool.
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl RoubitClient for LocalClient {
    async fn list_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.list_all_habits(LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.list_active_habits(LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn habit_checkin(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
        value: f64,
    ) -> Result<HabitLog, ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.record_check_in(habit_id, LOCAL_USER_ID, date, value, false)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn create_habit(
        &self,
        name: &str,
        habit_type: roubit_core::HabitType,
        target_value: f64,
        unit: Option<&str>,
        description: Option<&str>,
    ) -> Result<Habit, ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        let habit = Habit {
            id: Uuid::new_v4(),
            user_id: Some(LOCAL_USER_ID),
            name: name.to_string(),
            description: description.map(ToString::to_string),
            habit_type,
            target_value,
            unit: unit.map(ToString::to_string),
            frequency_days: 1,
            is_archived: false,
            color_hex: "#10b981".to_string(),
            current_streak: 0,
            longest_streak: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        repo.create_habit(LOCAL_USER_ID, &habit)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;
        Ok(habit)
    }

    async fn archive_habit(&self, habit_id: Uuid) -> Result<(), ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.archive_habit(habit_id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn delete_habit(&self, habit_id: Uuid) -> Result<(), ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.delete_habit(habit_id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn get_active_routine(&self) -> Result<Option<Routine>, ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.get_active_routine(LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_routine_blocks(&self, routine_id: Uuid) -> Result<Vec<TimeBlock>, ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.list_blocks_for_routine(routine_id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
    ) -> Result<(), ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.log_block_status(block_id, LOCAL_USER_ID, date, status, None)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_all_objectives(&self) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        repo.list_all_objectives(LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        repo.list_objectives_by_quarter(LOCAL_USER_ID, quarter)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_journal_metadata(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        repo.list_metadata_in_range(LOCAL_USER_ID, from, to)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    fn local_pool(&self) -> Option<&sqlx::SqlitePool> {
        Some(&self.pool)
    }
}
