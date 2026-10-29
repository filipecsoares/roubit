//! Core Domain Logic and Entity Models
//!
//! Subsystems:
//! - [`routine`]: Routine management and time-blocking (RF01)

pub mod routine;

pub use routine::{BlockStatus, DailyRoutineLog, Routine, TimeBlock};
