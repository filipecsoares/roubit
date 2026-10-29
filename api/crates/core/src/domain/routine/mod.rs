//! Routine Management Domain Module (RF01)
//!
//! Provides models, validation rules, and tracking engines for time-blocked routines.

pub mod models;
pub mod repository;
pub mod tracker;
pub mod validation;

pub use models::{BlockStatus, DailyRoutineLog, Routine, TimeBlock};
pub use repository::{
    create_sqlite_pool, RoutineRepository, RoutineRepositoryError, SqliteRoutineRepository,
};
pub use tracker::{CurrentRoutineProgress, RoutineProgressTracker};
pub use validation::{
    TimeBlockValidationError, TimeBlockValidator, MINIMUM_BLOCK_DURATION_MINUTES,
};
