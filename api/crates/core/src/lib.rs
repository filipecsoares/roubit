//! Roubit Core
//! Domain models, business logic, storage engines, and cryptographic services.

pub mod config;
pub mod crypto;
pub mod domain;
pub mod telemetry;

pub use config::AppConfig;
pub use crypto::{
    AesGcmCipher, CipherError, DerivedKey, EncryptedEnvelope, KdfError, KdfService, SecretBytes,
    SecretString,
};
pub use domain::{
    create_sqlite_pool, BlockStatus, CurrentRoutineProgress, DailyRoutineLog, Habit,
    HabitAnalyticsService, HabitAnalyticsSummary, HabitLog, HabitRepository, HabitRepositoryError,
    HabitType, HeatmapCell, KeyResult, Objective, Routine, RoutineProgressTracker,
    RoutineRepository, RoutineRepositoryError, SqliteHabitRepository, SqliteRoutineRepository,
    StreakCalculator, StreakMetrics, TimeBlock, TimeBlockValidationError, TimeBlockValidator,
    MINIMUM_BLOCK_DURATION_MINUTES,
};

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
