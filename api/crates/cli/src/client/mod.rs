//! Hybrid mode client adapter for local SQLite and remote REST API.

pub mod credentials;
pub mod factory;
pub mod local;
pub mod remote;

pub use factory::{ClientFactory, ClientMode};
pub use local::LocalClient;
pub use remote::HttpClient;

use async_trait::async_trait;
use chrono::NaiveDate;
use roubit_core::domain::{
    habit::models::{Habit, HabitLog},
    journal::models::JournalMetadata,
    routine::models::{Routine, TimeBlock},
    BlockStatus, ObjectiveWithKeyResults,
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

    /// Lists all objectives and associated key results.
    async fn list_all_objectives(&self) -> Result<Vec<ObjectiveWithKeyResults>, ClientError>;

    /// Lists objectives filtered by quarter.
    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, ClientError>;

    /// Lists metadata entries of encrypted journal reflections in a date range.
    async fn list_journal_metadata(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, ClientError>;
}
