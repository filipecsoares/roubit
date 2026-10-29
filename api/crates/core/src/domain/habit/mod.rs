//! Habit Tracking Domain Module (RF02)
//!
//! Provides models, check-in operations, and streak/consistency engines for habits.

pub mod models;
pub mod repository;

pub use models::{Habit, HabitLog, HabitType};
pub use repository::{HabitRepository, HabitRepositoryError, SqliteHabitRepository};
