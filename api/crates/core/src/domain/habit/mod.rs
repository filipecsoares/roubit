//! Habit Tracking Domain Module (RF02)
//!
//! Provides models, check-in operations, and streak/consistency engines for habits.

pub mod analytics;
pub mod models;
pub mod repository;
pub mod streak;

pub use analytics::{HabitAnalyticsService, HabitAnalyticsSummary, HeatmapCell};
pub use models::{Habit, HabitLog, HabitType};
pub use repository::{HabitRepository, HabitRepositoryError, SqliteHabitRepository};
pub use streak::{StreakCalculator, StreakMetrics};
