//! Roubit Core
//! Domain models, business logic, storage engines, and cryptographic services.

pub mod config;
pub mod crypto;
pub mod domain;
pub mod redis;
pub mod telemetry;

pub use config::AppConfig;
pub use crypto::{
    AesGcmCipher, CipherError, DerivedKey, EncryptedEnvelope, KdfError, KdfService, SecretBytes,
    SecretString,
};
pub use domain::{
    create_sqlite_pool, BlockStatus, CurrentRoutineProgress, DailyRoutineLog,
    DecryptedJournalEntry, EncryptedJournalEntry, Habit, HabitAnalyticsService,
    HabitAnalyticsSummary, HabitLog, HabitRepository, HabitRepositoryError, HabitType, HeatmapCell,
    JournalMetadata, JournalModelError, JournalRepository, JournalRepositoryError, JournalService,
    JournalServiceError, KeyResult, Objective, ObjectiveWithKeyResults, OkrProgressEngine,
    OkrRepository, OkrRepositoryError, OkrSyncService, Routine, RoutineProgressTracker,
    RoutineRepository, RoutineRepositoryError, SqliteHabitRepository, SqliteJournalRepository,
    SqliteOkrRepository, SqliteRoutineRepository, StreakCalculator, StreakMetrics, TimeBlock,
    TimeBlockValidationError, TimeBlockValidator, MINIMUM_BLOCK_DURATION_MINUTES,
};
pub use redis::{RedisError, RedisService};

/// Runs all embedded database migrations against the provided SQLite connection pool.
///
/// # Errors
/// Returns [`sqlx::migrate::MigrateError`] if applying migrations fails.
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub use telemetry::init_telemetry;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
