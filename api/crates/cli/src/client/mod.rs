//! Hybrid mode client adapter for local SQLite and remote REST API.

pub mod config;
pub mod credentials;
pub mod factory;
pub mod local;
pub mod remote;

pub use config::{CliConfigFile, CliConfigManager};
pub use credentials::{CredentialsManager, StoredCredentials};
pub use factory::{ClientFactory, ClientMode};
pub use local::LocalClient;
pub use remote::HttpClient;

use async_trait::async_trait;
use chrono::NaiveDate;
use roubit_core::domain::{
    habit::models::{Habit, HabitLog},
    journal::models::{EncryptedJournalEntry, JournalMetadata},
    routine::models::{Routine, TimeBlock},
    routine::CurrentRoutineProgress,
    BlockStatus, KeyResult, Objective, ObjectiveWithKeyResults,
};
use uuid::Uuid;

/// Errors produced during client operations in local or remote modes.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Failure during local domain execution or database operations.
    #[error("Local engine error: {0}")]
    Local(String),

    /// Failure in HTTP network transport.
    #[error("HTTP network error: {0}")]
    Http(#[from] reqwest::Error),

    /// Remote REST API returned non-success HTTP status.
    #[error("API returned error ({status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message or problem details payload.
        message: String,
    },
}

/// Abstract contract for Roubit client interactions (Local and Remote).
#[async_trait]
pub trait RoubitClient: Send + Sync {
    /// Lists all habits.
    async fn list_habits(&self) -> Result<Vec<Habit>, ClientError>;

    /// Lists only active (non-archived) habits.
    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError>;

    /// Records a check-in progress entry for a habit.
    async fn habit_checkin(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
        value: f64,
    ) -> Result<HabitLog, ClientError>;

    /// Creates a new habit definition.
    async fn create_habit(
        &self,
        name: &str,
        habit_type: roubit_core::HabitType,
        target_value: f64,
        unit: Option<&str>,
        description: Option<&str>,
    ) -> Result<Habit, ClientError>;

    /// Archives a habit by ID.
    async fn archive_habit(&self, habit_id: Uuid) -> Result<(), ClientError>;

    /// Deletes a habit by ID.
    async fn delete_habit(&self, habit_id: Uuid) -> Result<(), ClientError>;

    /// Fetches the currently active routine template.
    async fn get_active_routine(&self) -> Result<Option<Routine>, ClientError>;

    /// Lists scheduled time blocks for a routine template.
    async fn list_routine_blocks(&self, routine_id: Uuid) -> Result<Vec<TimeBlock>, ClientError>;

    /// Logs the execution status of a routine time block for a given date.
    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
    ) -> Result<(), ClientError>;

    /// Fetches active routine, scheduled blocks with statuses, and computed progress for a target date.
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
    >;

    /// Creates a new routine template with optional time blocks.
    async fn create_routine(
        &self,
        name: &str,
        blocks: &[TimeBlock],
        is_active: bool,
    ) -> Result<Routine, ClientError>;

    /// Sets a routine as the active daily template.
    async fn activate_routine(&self, routine_id: Uuid) -> Result<(), ClientError>;

    /// Deletes a routine template and associated time blocks.
    async fn delete_routine(&self, routine_id: Uuid) -> Result<(), ClientError>;

    /// Lists all objectives and associated key results.
    async fn list_all_objectives(&self) -> Result<Vec<ObjectiveWithKeyResults>, ClientError>;

    /// Lists objectives filtered by quarter.
    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, ClientError>;

    /// Creates a new objective.
    async fn create_objective(
        &self,
        title: &str,
        quarter: &str,
        description: Option<&str>,
    ) -> Result<Objective, ClientError>;

    /// Adds a key result to an objective.
    async fn create_key_result(
        &self,
        objective_id: Uuid,
        title: &str,
        target_value: f64,
        unit: Option<&str>,
        linked_habit_id: Option<Uuid>,
    ) -> Result<KeyResult, ClientError>;

    /// Deletes an objective.
    async fn delete_objective(&self, objective_id: Uuid) -> Result<(), ClientError>;

    /// Lists metadata entries of encrypted journal reflections in a date range.
    async fn list_journal_metadata(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, ClientError>;

    /// Fetches an encrypted journal entry by its unique ID.
    async fn get_journal_entry(
        &self,
        id: Uuid,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError>;

    /// Fetches an encrypted journal entry for a given calendar date.
    async fn get_journal_entry_by_date(
        &self,
        date: NaiveDate,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError>;

    /// Deletes an encrypted journal entry by calendar date. Returns true if deleted, false if not found.
    async fn delete_journal_entry_by_date(&self, date: NaiveDate) -> Result<bool, ClientError>;

    /// Saves an encrypted journal reflection.
    async fn save_journal(
        &self,
        date: NaiveDate,
        ciphertext_base64: &str,
        salt_base64: &str,
        nonce_base64: &str,
        mood: Option<i32>,
        tags: &[String],
    ) -> Result<(), ClientError>;

    /// Returns the underlying SQLite pool if running in local mode.
    fn local_pool(&self) -> Option<&sqlx::SqlitePool> {
        None
    }
}
