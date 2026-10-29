//! Domain Models for Routine Management and Time-Blocking
//!
//! Represents structured routines, discrete time blocks, and daily execution logs.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A high-level routine template (e.g. "Weekday Focus", "Weekend Flow").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct Routine {
    /// Unique identifier for the routine template.
    pub id: Uuid,

    /// Human-readable name of the routine.
    pub name: String,

    /// Optional markdown or plain text description.
    pub description: Option<String>,

    /// Whether this routine is actively in use.
    pub is_active: bool,

    /// UTC timestamp of creation.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of last modification.
    pub updated_at: DateTime<Utc>,
}

impl Routine {
    /// Creates a new `Routine` instance with generated timestamps and active status.
    #[must_use]
    pub fn new(name: impl Into<String>, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A scheduled time block within a routine template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct TimeBlock {
    /// Unique identifier for the time block.
    pub id: Uuid,

    /// Reference to parent routine template.
    pub routine_id: Uuid,

    /// Label or title of the activity (e.g., "Deep Work", "Exercise").
    pub name: String,

    /// Start time of the block (e.g., 08:30:00).
    pub start_time: NaiveTime,

    /// End time of the block (e.g., 10:00:00).
    pub end_time: NaiveTime,

    /// Functional category (e.g., "Focus", "Health", "Leisure", "Admin").
    pub category: String,

    /// Hexadecimal color for UI rendering (e.g., "#6366F1").
    pub color_hex: String,

    /// Display order index within the day's timeline.
    pub order_index: i32,

    /// UTC timestamp of creation.
    pub created_at: DateTime<Utc>,
}

impl TimeBlock {
    /// Default category assigned to new time blocks.
    pub const DEFAULT_CATEGORY: &'static str = "General";

    /// Default hex color assigned to new time blocks.
    pub const DEFAULT_COLOR: &'static str = "#6366F1";

    /// Creates a new `TimeBlock` instance.
    #[must_use]
    pub fn new(
        routine_id: Uuid,
        name: impl Into<String>,
        start_time: NaiveTime,
        end_time: NaiveTime,
        order_index: i32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            routine_id,
            name: name.into(),
            start_time,
            end_time,
            category: Self::DEFAULT_CATEGORY.to_string(),
            color_hex: Self::DEFAULT_COLOR.to_string(),
            order_index,
            created_at: Utc::now(),
        }
    }

    /// Calculates the duration of the time block in minutes.
    #[must_use]
    pub fn duration_minutes(&self) -> i64 {
        let diff = self.end_time - self.start_time;
        diff.num_minutes()
    }
}

/// Execution status of a scheduled time block for a specific calendar day.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BlockStatus {
    /// Block is scheduled but not yet reached or started.
    #[default]
    Pending,

    /// User is currently executing this time block.
    InProgress,

    /// Block was completed successfully.
    Completed,

    /// Block was intentionally or unavoidably skipped.
    Skipped,
}

impl BlockStatus {
    /// Returns the lowercase string representation matching database constraints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }
}

impl std::fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Execution check-in record tracking completion of a time block on a specific date.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, FromRow)]
pub struct DailyRoutineLog {
    /// Unique identifier for the log entry.
    pub id: Uuid,

    /// Target time block identifier.
    pub time_block_id: Uuid,

    /// Calendar date of execution (YYYY-MM-DD).
    pub date: NaiveDate,

    /// Status of execution.
    pub status: BlockStatus,

    /// Real start timestamp (UTC), if recorded.
    pub started_at: Option<DateTime<Utc>>,

    /// Real completion timestamp (UTC), if recorded.
    pub completed_at: Option<DateTime<Utc>>,

    /// Optional reflection or deviation notes.
    pub notes: Option<String>,
}

impl DailyRoutineLog {
    /// Creates a new pending check-in record for a time block on a given date.
    #[must_use]
    pub fn new_pending(time_block_id: Uuid, date: NaiveDate) -> Self {
        Self {
            id: Uuid::new_v4(),
            time_block_id,
            date,
            status: BlockStatus::Pending,
            started_at: None,
            completed_at: None,
            notes: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_routine_creation_defaults() {
        let routine = Routine::new("Deep Work Morning", Some("High cognitive load".into()));
        assert_eq!(routine.name, "Deep Work Morning");
        assert_eq!(routine.description.as_deref(), Some("High cognitive load"));
        assert!(routine.is_active);
    }

    #[test]
    fn test_time_block_duration_minutes() {
        let routine_id = Uuid::new_v4();
        let start = NaiveTime::from_hms_opt(8, 30, 0).expect("valid start time");
        let end = NaiveTime::from_hms_opt(10, 15, 0).expect("valid end time");

        let block = TimeBlock::new(routine_id, "Coding Session", start, end, 1);
        assert_eq!(block.duration_minutes(), 105);
        assert_eq!(block.category, TimeBlock::DEFAULT_CATEGORY);
        assert_eq!(block.color_hex, TimeBlock::DEFAULT_COLOR);
    }

    #[test]
    fn test_block_status_display_and_as_str() {
        assert_eq!(BlockStatus::Pending.as_str(), "pending");
        assert_eq!(BlockStatus::InProgress.as_str(), "in_progress");
        assert_eq!(BlockStatus::Completed.as_str(), "completed");
        assert_eq!(BlockStatus::Skipped.as_str(), "skipped");

        assert_eq!(format!("{}", BlockStatus::Completed), "completed");
        assert_eq!(BlockStatus::default(), BlockStatus::Pending);
    }

    #[test]
    fn test_daily_routine_log_pending() {
        let block_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");

        let log = DailyRoutineLog::new_pending(block_id, date);
        assert_eq!(log.time_block_id, block_id);
        assert_eq!(log.date, date);
        assert_eq!(log.status, BlockStatus::Pending);
        assert!(log.started_at.is_none());
        assert!(log.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_routine_sqlite_migrations_and_foreign_keys() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite must connect");

        // Enable foreign keys in SQLite
        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await
            .expect("pragma foreign_keys must execute");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations must succeed");

        // 1. Insert Routine
        let routine = Routine::new("Daily Habits", Some("Consistency routine".into()));
        sqlx::query(
            "INSERT INTO routines (id, name, description, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(routine.id)
        .bind(&routine.name)
        .bind(&routine.description)
        .bind(routine.is_active)
        .bind(routine.created_at)
        .bind(routine.updated_at)
        .execute(&pool)
        .await
        .expect("routine insert must succeed");

        // 2. Insert TimeBlock
        let start = NaiveTime::from_hms_opt(9, 0, 0).expect("valid start time");
        let end = NaiveTime::from_hms_opt(11, 0, 0).expect("valid end time");
        let block = TimeBlock::new(routine.id, "Focus Block", start, end, 0);

        sqlx::query(
            "INSERT INTO time_blocks (id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(block.id)
        .bind(block.routine_id)
        .bind(&block.name)
        .bind(block.start_time)
        .bind(block.end_time)
        .bind(&block.category)
        .bind(&block.color_hex)
        .bind(block.order_index)
        .bind(block.created_at)
        .execute(&pool)
        .await
        .expect("time block insert must succeed");

        // 3. Query Routine and TimeBlock
        let fetched_routine: Routine = sqlx::query_as("SELECT id, name, description, is_active, created_at, updated_at FROM routines WHERE id = ?")
            .bind(routine.id)
            .fetch_one(&pool)
            .await
            .expect("fetch routine must succeed");

        assert_eq!(fetched_routine.id, routine.id);
        assert_eq!(fetched_routine.name, "Daily Habits");
        assert!(fetched_routine.is_active);

        let fetched_block: TimeBlock = sqlx::query_as("SELECT id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at FROM time_blocks WHERE id = ?")
            .bind(block.id)
            .fetch_one(&pool)
            .await
            .expect("fetch time block must succeed");

        assert_eq!(fetched_block.id, block.id);
        assert_eq!(fetched_block.start_time, start);
        assert_eq!(fetched_block.end_time, end);

        // 4. Test Foreign Key Cascade Deletion
        sqlx::query("DELETE FROM routines WHERE id = ?")
            .bind(routine.id)
            .execute(&pool)
            .await
            .expect("delete routine must succeed");

        let blocks_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM time_blocks WHERE routine_id = ?")
                .bind(routine.id)
                .fetch_one(&pool)
                .await
                .expect("count must succeed");

        assert_eq!(
            blocks_count.0, 0,
            "TimeBlock must be cascaded on routine deletion"
        );
    }
}
