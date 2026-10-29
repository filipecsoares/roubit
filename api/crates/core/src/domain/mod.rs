//! Core Domain Logic and Entity Models
//!
//! Subsystems:
//! - [`routine`]: Routine management and time-blocking (RF01)

pub mod habit;
pub mod okr;
pub mod routine;

pub use habit::{
    Habit, HabitAnalyticsService, HabitAnalyticsSummary, HabitLog, HabitRepository,
    HabitRepositoryError, HabitType, HeatmapCell, SqliteHabitRepository, StreakCalculator,
    StreakMetrics,
};

pub use okr::{
    KeyResult, Objective, ObjectiveWithKeyResults, OkrProgressEngine, OkrRepository,
    OkrRepositoryError, OkrSyncService, SqliteOkrRepository,
};

pub use routine::{
    create_sqlite_pool, BlockStatus, CurrentRoutineProgress, DailyRoutineLog, Routine,
    RoutineProgressTracker, RoutineRepository, RoutineRepositoryError, SqliteRoutineRepository,
    TimeBlock, TimeBlockValidationError, TimeBlockValidator, MINIMUM_BLOCK_DURATION_MINUTES,
};
