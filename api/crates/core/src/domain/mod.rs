//! Core Domain Logic and Entity Models
//!
//! Subsystems:
//! - [`routine`]: Routine management and time-blocking (RF01)

pub mod routine;

pub use routine::{
    create_sqlite_pool, BlockStatus, DailyRoutineLog, Routine, RoutineRepository,
    RoutineRepositoryError, SqliteRoutineRepository, TimeBlock,
};
