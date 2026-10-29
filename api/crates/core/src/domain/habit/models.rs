//! Habit Tracker Domain Entities and Value Objects (RF02)
//!
//! Provides models for binary, numeric, and timer-based habit tracking,
//! frequency bitmasks, and daily completion logs.

use std::fmt::{Display, Formatter};

use chrono::{DateTime, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Measurement modality for a habit.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum HabitType {
    /// Binary completion (yes/no, e.g., "Floss teeth").
    #[default]
    Binary,

    /// Quantitative target with units (e.g., "Drink 2500 ml", "Read 20 pages").
    Numeric,

    /// Duration-based target in minutes (e.g., "Meditate for 15 minutes").
    Timer,
}

impl HabitType {
    /// String slice representation matching SQLite constraint.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Numeric => "numeric",
            Self::Timer => "timer",
        }
    }
}

impl Display for HabitType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A recurring habit definition with measurement targets and scheduled frequency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
#[allow(clippy::struct_field_names)]
pub struct Habit {
    /// Unique identifier for the habit.
    pub id: Uuid,

    /// Title or label of the habit.
    pub name: String,

    /// Detailed description or motivation for the habit.
    pub description: Option<String>,

    /// Measurement modality (binary, numeric, timer).
    pub habit_type: HabitType,

    /// Target value required for full completion (1.0 for binary).
    pub target_value: f64,

    /// Measurement unit label (e.g., "ml", "pages", "min"), if applicable.
    pub unit: Option<String>,

    /// 7-day bitmask representation of scheduled days (1=Mon, 2=Tue, 4=Wed, 8=Thu, 16=Fri, 32=Sat, 64=Sun; 127=Daily).
    pub frequency_days: u8,

    /// Hexadecimal color for UI card and heatmap rendering (e.g., "#10B981").
    pub color_hex: String,

    /// Whether this habit is archived and hidden from daily check-ins.
    pub is_archived: bool,

    /// UTC timestamp of creation.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of last modification.
    pub updated_at: DateTime<Utc>,
}

impl Habit {
    /// Default green accent color for habits.
    pub const DEFAULT_COLOR: &'static str = "#10B981";

    /// Bitmask for all 7 days of the week (`1 + 2 + 4 + 8 + 16 + 32 + 64 = 127`).
    pub const FREQUENCY_DAILY: u8 = 127;

    /// Bitmask for weekdays Mon-Fri (`1 + 2 + 4 + 8 + 16 = 31`).
    pub const FREQUENCY_WEEKDAYS: u8 = 31;

    /// Bitmask for weekends Sat-Sun (`32 + 64 = 96`).
    pub const FREQUENCY_WEEKENDS: u8 = 96;

    /// Creates a new binary habit scheduled daily by default.
    #[must_use]
    pub fn new_binary(name: impl Into<String>, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description,
            habit_type: HabitType::Binary,
            target_value: 1.0,
            unit: None,
            frequency_days: Self::FREQUENCY_DAILY,
            color_hex: Self::DEFAULT_COLOR.to_string(),
            is_archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new numeric habit with a quantitative target and unit.
    #[must_use]
    pub fn new_numeric(
        name: impl Into<String>,
        description: Option<String>,
        target_value: f64,
        unit: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description,
            habit_type: HabitType::Numeric,
            target_value,
            unit: Some(unit.into()),
            frequency_days: Self::FREQUENCY_DAILY,
            color_hex: Self::DEFAULT_COLOR.to_string(),
            is_archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new timer habit with a duration target in minutes.
    #[must_use]
    pub fn new_timer(
        name: impl Into<String>,
        description: Option<String>,
        target_minutes: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description,
            habit_type: HabitType::Timer,
            target_value: target_minutes,
            unit: Some("min".to_string()),
            frequency_days: Self::FREQUENCY_DAILY,
            color_hex: Self::DEFAULT_COLOR.to_string(),
            is_archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Evaluates whether this habit is scheduled to occur on the specified [`Weekday`].
    #[must_use]
    pub const fn is_scheduled_on(&self, weekday: Weekday) -> bool {
        let bit = 1u8 << weekday.num_days_from_monday();
        (self.frequency_days & bit) != 0
    }
}

/// Daily execution check-in log record for a habit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct HabitLog {
    /// Unique identifier for the log entry.
    pub id: Uuid,

    /// Reference to parent habit.
    pub habit_id: Uuid,

    /// Calendar date of the check-in (YYYY-MM-DD).
    pub log_date: NaiveDate,

    /// Logged quantitative value (1.0 for binary completed, ml/pages for numeric, minutes for timer).
    pub actual_value: f64,

    /// Whether this check-in satisfied the habit's completion criteria.
    pub is_completed: bool,

    /// Optional contextual notes or reflections about this check-in.
    pub notes: Option<String>,

    /// UTC timestamp of logging.
    pub logged_at: DateTime<Utc>,
}

impl HabitLog {
    /// Creates a new `HabitLog` entry.
    #[must_use]
    pub fn new(
        habit_id: Uuid,
        log_date: NaiveDate,
        actual_value: f64,
        is_completed: bool,
        notes: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            habit_id,
            log_date,
            actual_value,
            is_completed,
            notes,
            logged_at: Utc::now(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_habit_type_display_and_as_str() {
        assert_eq!(HabitType::Binary.as_str(), "binary");
        assert_eq!(HabitType::Numeric.as_str(), "numeric");
        assert_eq!(HabitType::Timer.as_str(), "timer");

        assert_eq!(HabitType::Binary.to_string(), "binary");
        assert_eq!(HabitType::Numeric.to_string(), "numeric");
        assert_eq!(HabitType::Timer.to_string(), "timer");
    }

    #[test]
    fn test_habit_type_serde_roundtrip() {
        let json = serde_json::to_string(&HabitType::Numeric).expect("serialize");
        assert_eq!(json, "\"numeric\"");
        let deserialized: HabitType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, HabitType::Numeric);
    }

    #[test]
    fn test_habit_creation_defaults() {
        let binary = Habit::new_binary("Floss Teeth", Some("Dental health".into()));
        assert_eq!(binary.habit_type, HabitType::Binary);
        assert_eq!(binary.target_value, 1.0);
        assert!(binary.unit.is_none());
        assert_eq!(binary.frequency_days, Habit::FREQUENCY_DAILY);
        assert!(!binary.is_archived);

        let numeric = Habit::new_numeric("Water Intake", None, 2500.0, "ml");
        assert_eq!(numeric.habit_type, HabitType::Numeric);
        assert_eq!(numeric.target_value, 2500.0);
        assert_eq!(numeric.unit.as_deref(), Some("ml"));

        let timer = Habit::new_timer("Meditation", None, 15.0);
        assert_eq!(timer.habit_type, HabitType::Timer);
        assert_eq!(timer.target_value, 15.0);
        assert_eq!(timer.unit.as_deref(), Some("min"));
    }

    #[test]
    fn test_frequency_bitmask_evaluation() {
        let mut habit = Habit::new_binary("Exercise", None);

        // 1. Daily frequency
        habit.frequency_days = Habit::FREQUENCY_DAILY;
        assert!(habit.is_scheduled_on(Weekday::Mon));
        assert!(habit.is_scheduled_on(Weekday::Wed));
        assert!(habit.is_scheduled_on(Weekday::Sun));

        // 2. Weekdays only (Mon-Fri)
        habit.frequency_days = Habit::FREQUENCY_WEEKDAYS;
        assert!(habit.is_scheduled_on(Weekday::Mon));
        assert!(habit.is_scheduled_on(Weekday::Fri));
        assert!(!habit.is_scheduled_on(Weekday::Sat));
        assert!(!habit.is_scheduled_on(Weekday::Sun));

        // 3. Weekends only (Sat-Sun)
        habit.frequency_days = Habit::FREQUENCY_WEEKENDS;
        assert!(!habit.is_scheduled_on(Weekday::Mon));
        assert!(habit.is_scheduled_on(Weekday::Sat));
        assert!(habit.is_scheduled_on(Weekday::Sun));

        // 4. Custom: Mon, Wed, Fri (1 + 4 + 16 = 21)
        habit.frequency_days = 1 | 4 | 16;
        assert!(habit.is_scheduled_on(Weekday::Mon));
        assert!(!habit.is_scheduled_on(Weekday::Tue));
        assert!(habit.is_scheduled_on(Weekday::Wed));
        assert!(!habit.is_scheduled_on(Weekday::Thu));
        assert!(habit.is_scheduled_on(Weekday::Fri));
        assert!(!habit.is_scheduled_on(Weekday::Sat));
        assert!(!habit.is_scheduled_on(Weekday::Sun));
    }

    #[test]
    fn test_habit_log_creation() {
        let habit_id = Uuid::new_v4();
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");
        let log = HabitLog::new(
            habit_id,
            date,
            2500.0,
            true,
            Some("Hydrated throughout the day".into()),
        );

        assert_eq!(log.habit_id, habit_id);
        assert_eq!(log.log_date, date);
        assert_eq!(log.actual_value, 2500.0);
        assert!(log.is_completed);
        assert_eq!(log.notes.as_deref(), Some("Hydrated throughout the day"));
    }

    #[tokio::test]
    async fn test_habit_sqlite_migrations_and_foreign_keys() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite must connect");

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await
            .expect("pragma foreign_keys must execute");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations must succeed");

        // 1. Insert Habit
        let habit = Habit::new_numeric("Read Books", Some("Reading daily".into()), 20.0, "pages");
        sqlx::query(
            r"
            INSERT INTO habits (id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(habit.id)
        .bind(&habit.name)
        .bind(&habit.description)
        .bind(habit.habit_type)
        .bind(habit.target_value)
        .bind(&habit.unit)
        .bind(habit.frequency_days)
        .bind(&habit.color_hex)
        .bind(habit.is_archived)
        .bind(habit.created_at)
        .bind(habit.updated_at)
        .execute(&pool)
        .await
        .expect("insert habit succeeds");

        // 2. Insert HabitLog
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).expect("valid date");
        let log = HabitLog::new(habit.id, date, 25.0, true, Some("Great chapter".into()));

        sqlx::query(
            r"
            INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(log.id)
        .bind(log.habit_id)
        .bind(log.log_date)
        .bind(log.actual_value)
        .bind(log.is_completed)
        .bind(&log.notes)
        .bind(log.logged_at)
        .execute(&pool)
        .await
        .expect("insert habit log succeeds");

        // 3. Query Habit using FromRow
        let fetched_habit: Habit = sqlx::query_as(
            "SELECT id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at FROM habits WHERE id = ?",
        )
        .bind(habit.id)
        .fetch_one(&pool)
        .await
        .expect("fetch habit succeeds");

        assert_eq!(fetched_habit.id, habit.id);
        assert_eq!(fetched_habit.habit_type, HabitType::Numeric);
        assert_eq!(fetched_habit.target_value, 20.0);
        assert_eq!(fetched_habit.unit.as_deref(), Some("pages"));

        // 4. Query HabitLog using FromRow
        let fetched_log: HabitLog = sqlx::query_as(
            "SELECT id, habit_id, log_date, actual_value, is_completed, notes, logged_at FROM habit_logs WHERE id = ?",
        )
        .bind(log.id)
        .fetch_one(&pool)
        .await
        .expect("fetch habit log succeeds");

        assert_eq!(fetched_log.id, log.id);
        assert_eq!(fetched_log.actual_value, 25.0);
        assert!(fetched_log.is_completed);

        // 5. Test Unique Constraint on (habit_id, log_date)
        let dup_log = HabitLog::new(habit.id, date, 30.0, true, None);
        let dup_result = sqlx::query(
            r"
            INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(dup_log.id)
        .bind(dup_log.habit_id)
        .bind(dup_log.log_date)
        .bind(dup_log.actual_value)
        .bind(dup_log.is_completed)
        .bind(&dup_log.notes)
        .bind(dup_log.logged_at)
        .execute(&pool)
        .await;

        assert!(
            dup_result.is_err(),
            "Duplicate (habit_id, log_date) must be rejected"
        );

        // 6. Test Foreign Key Cascade Deletion
        sqlx::query("DELETE FROM habits WHERE id = ?")
            .bind(habit.id)
            .execute(&pool)
            .await
            .expect("delete habit succeeds");

        let logs_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM habit_logs WHERE habit_id = ?")
                .bind(habit.id)
                .fetch_one(&pool)
                .await
                .expect("count query succeeds");

        assert_eq!(
            logs_count.0, 0,
            "Habit logs must cascade delete on habit deletion"
        );
    }
}
