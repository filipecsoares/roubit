//! Core Domain Logic and Entity Models
//!
//! Subsystems:
//! - [`routine`]: Routine management and time-blocking (RF01)

pub mod habit;
pub mod journal;
pub mod okr;
pub mod routine;
pub mod user;

/// Default system / local user identifier for CLI local operations and fallback context.
pub const LOCAL_USER_ID: uuid::Uuid = uuid::Uuid::from_u128(1);

/// Seeds a user record for testing referential integrity and multi-tenancy.
///
/// # Errors
///
/// Returns [`sqlx::Error`] if database insertion fails.
pub async fn seed_test_user(
    pool: &sqlx::SqlitePool,
    user_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
    )
    .bind(user_id)
    .bind(format!("user-{user_id}@example.com"))
    .bind("test-hash")
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub use journal::{
    deserialize_tags, serialize_tags, DecryptedJournalEntry, EncryptedJournalEntry,
    JournalMetadata, JournalModelError, JournalRepository, JournalRepositoryError, JournalService,
    JournalServiceError, PgJournalRepository, SqliteJournalRepository,
};

pub use habit::{
    Habit, HabitAnalyticsService, HabitAnalyticsSummary, HabitLog, HabitRepository,
    HabitRepositoryError, HabitType, HeatmapCell, PgHabitRepository, SqliteHabitRepository,
    StreakCalculator, StreakMetrics,
};

pub use okr::{
    KeyResult, Objective, ObjectiveWithKeyResults, OkrProgressEngine, OkrRepository,
    OkrRepositoryError, OkrSyncService, PgOkrRepository, SqliteOkrRepository,
};

pub use routine::{
    create_sqlite_pool, BlockStatus, CurrentRoutineProgress, DailyRoutineLog, PgRoutineRepository,
    Routine, RoutineProgressTracker, RoutineRepository, RoutineRepositoryError,
    SqliteRoutineRepository, TimeBlock, TimeBlockValidationError, TimeBlockValidator,
    MINIMUM_BLOCK_DURATION_MINUTES,
};

pub use user::{PgUserRepository, SqliteUserRepository, User, UserRepository, UserRepositoryError};
