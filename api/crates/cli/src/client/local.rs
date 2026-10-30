//! Local client adapter delegating directly to SQLite repositories.

use async_trait::async_trait;
use chrono::NaiveDate;
use roubit_core::domain::{
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
        repo.list_all_habits()
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let repo = SqliteHabitRepository::new(self.pool.clone());
        repo.list_active_habits()
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
        repo.record_check_in(habit_id, date, value, false)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn get_active_routine(&self) -> Result<Option<Routine>, ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.get_active_routine()
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_routine_blocks(&self, routine_id: Uuid) -> Result<Vec<TimeBlock>, ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.list_blocks_for_routine(routine_id)
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
        repo.log_block_status(block_id, date, status, None)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_all_objectives(&self) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        repo.list_all_objectives()
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        repo.list_objectives_by_quarter(quarter)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn list_journal_metadata(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        repo.list_metadata_in_range(from, to)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }
}
