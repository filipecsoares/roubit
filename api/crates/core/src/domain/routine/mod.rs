//! Routine Management Domain Module (RF01)
//!
//! Provides models, validation rules, and tracking engines for time-blocked routines.

pub mod models;
pub mod repository;

pub use models::{BlockStatus, DailyRoutineLog, Routine, TimeBlock};
pub use repository::{
    create_sqlite_pool, RoutineRepository, RoutineRepositoryError, SqliteRoutineRepository,
};
