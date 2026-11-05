//! Local client adapter delegating directly to SQLite repositories.

use async_trait::async_trait;
use base64::prelude::*;
use chrono::NaiveDate;
use roubit_core::{
    domain::{
        habit::{
            models::{Habit, HabitLog},
            HabitRepository, SqliteHabitRepository,
        },
        journal::{
            models::{EncryptedJournalEntry, JournalMetadata},
            JournalRepository, SqliteJournalRepository,
        },
        okr::{OkrRepository, SqliteOkrRepository},
        routine::{
            models::{Routine, TimeBlock},
            CurrentRoutineProgress, RoutineRepository, SqliteRoutineRepository,
        },
        BlockStatus, KeyResult, Objective, ObjectiveWithKeyResults,
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

    async fn get_active_routine_progress(
        &self,
        date: NaiveDate,
    ) -> Result<
        Option<(
            Routine,
            Vec<(TimeBlock, BlockStatus)>,
            CurrentRoutineProgress,
        )>,
        ClientError,
    > {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        let routine_opt = repo
            .get_active_routine(LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        let Some(routine) = routine_opt else {
            return Ok(None);
        };

        let blocks = repo
            .list_blocks_for_routine(routine.id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        let mut block_statuses = Vec::with_capacity(blocks.len());
        for block in &blocks {
            let log_opt = repo
                .get_daily_log(block.id, LOCAL_USER_ID, date)
                .await
                .map_err(|e| ClientError::Local(e.to_string()))?;
            let status = log_opt.map_or(BlockStatus::Pending, |l| l.status);
            block_statuses.push((block.clone(), status));
        }

        let now_time = chrono::Local::now().time();
        let progress = roubit_core::RoutineProgressTracker::compute_progress(
            &blocks,
            &block_statuses,
            now_time,
        );

        Ok(Some((routine, block_statuses, progress)))
    }

    async fn create_routine(
        &self,
        name: &str,
        blocks: &[TimeBlock],
        is_active: bool,
    ) -> Result<Routine, ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        let routine_id = Uuid::new_v4();
        let routine = Routine {
            id: routine_id,
            user_id: Some(LOCAL_USER_ID),
            name: name.to_string(),
            description: None,
            is_active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let mut assigned_blocks = blocks.to_vec();
        for b in &mut assigned_blocks {
            b.routine_id = routine_id;
        }

        repo.create_routine_with_blocks(LOCAL_USER_ID, &routine, &assigned_blocks)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        if is_active {
            repo.set_active_routine(routine_id, LOCAL_USER_ID)
                .await
                .map_err(|e| ClientError::Local(e.to_string()))?;
        }

        Ok(routine)
    }

    async fn activate_routine(&self, routine_id: Uuid) -> Result<(), ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.set_active_routine(routine_id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn delete_routine(&self, routine_id: Uuid) -> Result<(), ClientError> {
        let repo = SqliteRoutineRepository::new(self.pool.clone());
        repo.delete_routine(routine_id, LOCAL_USER_ID)
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

    async fn create_objective(
        &self,
        title: &str,
        quarter: &str,
        description: Option<&str>,
    ) -> Result<Objective, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        let today = chrono::Local::now().date_naive();
        let quarter_end = today + chrono::Duration::days(90);
        let objective = Objective::new(
            title.to_string(),
            description.map(ToString::to_string),
            quarter.to_string(),
            today,
            quarter_end,
        )
        .with_user_id(LOCAL_USER_ID);

        repo.create_objective(LOCAL_USER_ID, &objective)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        Ok(objective)
    }

    async fn create_key_result(
        &self,
        objective_id: Uuid,
        title: &str,
        target_value: f64,
        unit: Option<&str>,
        linked_habit_id: Option<Uuid>,
    ) -> Result<KeyResult, ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        let unit_str = unit.unwrap_or("%").to_string();
        let kr = if let Some(h_id) = linked_habit_id {
            KeyResult::new_habit_linked(
                objective_id,
                h_id,
                title.to_string(),
                target_value,
                unit_str,
                1.0,
            )
        } else {
            KeyResult::new_numeric(
                objective_id,
                title.to_string(),
                0.0,
                target_value,
                unit_str,
                1.0,
            )
        };

        repo.create_key_result(LOCAL_USER_ID, &kr)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        Ok(kr)
    }

    async fn delete_objective(&self, objective_id: Uuid) -> Result<(), ClientError> {
        let repo = SqliteOkrRepository::new(self.pool.clone());
        repo.delete_objective(objective_id, LOCAL_USER_ID)
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

    async fn get_journal_entry(
        &self,
        id: Uuid,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        repo.get_entry_by_id(id, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn get_journal_entry_by_date(
        &self,
        date: NaiveDate,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        repo.get_entry_by_date(date, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))
    }

    async fn delete_journal_entry_by_date(&self, date: NaiveDate) -> Result<bool, ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        if let Some(entry) = repo
            .get_entry_by_date(date, LOCAL_USER_ID)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?
        {
            repo.delete_entry(entry.metadata.id, LOCAL_USER_ID)
                .await
                .map_err(|e| ClientError::Local(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn save_journal(
        &self,
        date: NaiveDate,
        ciphertext_base64: &str,
        salt_base64: &str,
        nonce_base64: &str,
        mood: Option<i32>,
        tags: &[String],
    ) -> Result<(), ClientError> {
        let repo = SqliteJournalRepository::new(self.pool.clone());
        let salt = BASE64_STANDARD
            .decode(salt_base64)
            .map_err(|e| ClientError::Local(format!("invalid salt base64: {e}")))?;
        let nonce = BASE64_STANDARD
            .decode(nonce_base64)
            .map_err(|e| ClientError::Local(format!("invalid nonce base64: {e}")))?;
        let ciphertext = BASE64_STANDARD
            .decode(ciphertext_base64)
            .map_err(|e| ClientError::Local(format!("invalid ciphertext base64: {e}")))?;

        let mood_u8 = mood.and_then(|m| u8::try_from(m).ok());
        let metadata = roubit_core::domain::journal::models::JournalMetadata::new(
            date,
            mood_u8,
            tags.to_vec(),
            ciphertext.len(),
        )
        .map_err(|e| ClientError::Local(e.to_string()))?
        .with_user_id(LOCAL_USER_ID);

        let salt_arr: [u8; 16] = salt
            .try_into()
            .map_err(|_| ClientError::Local("salt must be 16 bytes".into()))?;
        let nonce_arr: [u8; 12] = nonce
            .try_into()
            .map_err(|_| ClientError::Local("nonce must be 12 bytes".into()))?;

        let entry = roubit_core::domain::journal::models::EncryptedJournalEntry::new(
            metadata, salt_arr, nonce_arr, ciphertext,
        )
        .map_err(|e| ClientError::Local(e.to_string()))?;

        repo.save_entry(LOCAL_USER_ID, &entry)
            .await
            .map_err(|e| ClientError::Local(e.to_string()))?;

        Ok(())
    }

    fn local_pool(&self) -> Option<&sqlx::SqlitePool> {
        Some(&self.pool)
    }
}
