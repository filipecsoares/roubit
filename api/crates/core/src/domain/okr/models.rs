//! Domain models for Objectives and Key Results (RF03).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// An overarching quarterly or annual aspirational goal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct Objective {
    /// Unique identifier for the objective.
    pub id: Uuid,

    /// Short title or statement of the aspirational goal.
    pub title: String,

    /// Optional detailed description or strategic motivation.
    pub description: Option<String>,

    /// Planning horizon or quarter (e.g., "2026-Q4" or "2026-Annual").
    pub quarter: String,

    /// Inclusive start date for the objective cycle.
    pub start_date: NaiveDate,

    /// Inclusive target end date for the objective cycle.
    pub end_date: NaiveDate,

    /// Aggregate progress percentage from 0.0 to 100.0 (weighted average of Key Results).
    pub progress_percentage: f32,

    /// Whether the objective has been marked as completed.
    pub is_completed: bool,

    /// UTC timestamp of creation.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of the last update.
    pub updated_at: DateTime<Utc>,
}

impl Objective {
    /// Creates a new `Objective` with zero initial progress.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        description: Option<String>,
        quarter: impl Into<String>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description,
            quarter: quarter.into(),
            start_date,
            end_date,
            progress_percentage: 0.0,
            is_completed: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Recalculates the objective's aggregate progress percentage as the weighted
    /// average of its child [`KeyResult`] items.
    #[allow(clippy::suboptimal_flops)]
    pub fn update_progress(&mut self, key_results: &[KeyResult]) {
        if key_results.is_empty() {
            self.progress_percentage = 0.0;
            self.is_completed = false;
            self.updated_at = Utc::now();
            return;
        }

        let mut total_weight = 0.0f32;
        let mut weighted_sum = 0.0f32;

        for kr in key_results {
            let weight = if kr.weight <= 0.0 { 1.0 } else { kr.weight };
            total_weight += weight;
            weighted_sum += kr.progress_percentage * weight;
        }

        if total_weight > 0.0 {
            self.progress_percentage = (weighted_sum / total_weight).clamp(0.0, 100.0);
        } else {
            self.progress_percentage = 0.0;
        }

        self.is_completed = self.progress_percentage >= 100.0;
        self.updated_at = Utc::now();
    }
}

/// A measurable target contributing towards an [`Objective`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct KeyResult {
    /// Unique identifier for the key result.
    pub id: Uuid,

    /// Foreign key reference to parent [`Objective`].
    pub objective_id: Uuid,

    /// Optional foreign key link to an operational habit whose check-ins drive this KR.
    pub linked_habit_id: Option<Uuid>,

    /// Descriptive title of the outcome to be achieved.
    pub title: String,

    /// Baseline starting metric value.
    pub initial_value: f64,

    /// Target target metric value.
    pub target_value: f64,

    /// Current achieved value towards the target.
    pub current_value: f64,

    /// Unit of measure (e.g., "km", "books", "check-ins", "kg").
    pub unit: String,

    /// Relative weight of this KR when computing the parent Objective's progress (e.g. 1.0).
    pub weight: f32,

    /// Calculated completion percentage from 0.0 to 100.0.
    pub progress_percentage: f32,

    /// UTC timestamp of creation.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of the last update.
    pub updated_at: DateTime<Utc>,
}

impl KeyResult {
    /// Creates a standalone numeric [`KeyResult`].
    #[must_use]
    pub fn new_numeric(
        objective_id: Uuid,
        title: impl Into<String>,
        initial_value: f64,
        target_value: f64,
        unit: impl Into<String>,
        weight: f32,
    ) -> Self {
        let now = Utc::now();
        let progress = Self::calculate_progress(initial_value, target_value, initial_value);
        Self {
            id: Uuid::new_v4(),
            objective_id,
            linked_habit_id: None,
            title: title.into(),
            initial_value,
            target_value,
            current_value: initial_value,
            unit: unit.into(),
            weight: if weight <= 0.0 { 1.0 } else { weight },
            progress_percentage: progress,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a [`KeyResult`] linked to an active daily [`Habit`].
    #[must_use]
    pub fn new_habit_linked(
        objective_id: Uuid,
        linked_habit_id: Uuid,
        title: impl Into<String>,
        target_value: f64,
        unit: impl Into<String>,
        weight: f32,
    ) -> Self {
        let now = Utc::now();
        let progress = Self::calculate_progress(0.0, target_value, 0.0);
        Self {
            id: Uuid::new_v4(),
            objective_id,
            linked_habit_id: Some(linked_habit_id),
            title: title.into(),
            initial_value: 0.0,
            target_value,
            current_value: 0.0,
            unit: unit.into(),
            weight: if weight <= 0.0 { 1.0 } else { weight },
            progress_percentage: progress,
            created_at: now,
            updated_at: now,
        }
    }

    /// Pure function calculating progress percentage from initial, target, and current values.
    /// Safely handles division by zero and clamps output to `[0.0, 100.0]`.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_progress(initial_value: f64, target_value: f64, current_value: f64) -> f32 {
        let delta = target_value - initial_value;
        if delta.abs() < f64::EPSILON {
            if current_value >= target_value {
                100.0
            } else {
                0.0
            }
        } else {
            let raw_progress = ((current_value - initial_value) / delta * 100.0) as f32;
            raw_progress.clamp(0.0, 100.0)
        }
    }

    /// Updates the current achieved metric value and recalculates progress percentage.
    pub fn update_current_value(&mut self, new_value: f64) {
        self.current_value = new_value;
        self.progress_percentage =
            Self::calculate_progress(self.initial_value, self.target_value, new_value);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::similar_names, clippy::too_many_lines)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_key_result_progress_calculation() {
        // Normal progression: 0 to 100km, currently at 45km -> 45.0%
        let progress = KeyResult::calculate_progress(0.0, 100.0, 45.0);
        assert!((progress - 45.0).abs() < f32::EPSILON);

        // Target exceeded: 0 to 100km, currently at 120km -> capped at 100.0%
        let progress_capped = KeyResult::calculate_progress(0.0, 100.0, 120.0);
        assert!((progress_capped - 100.0).abs() < f32::EPSILON);

        // Inverse progression (e.g. weight loss from 90kg down to 80kg):
        // initial: 90, target: 80, current: 85 -> delta is -10, current-initial is -5 -> 50%
        let progress_desc = KeyResult::calculate_progress(90.0, 80.0, 85.0);
        assert!((progress_desc - 50.0).abs() < f32::EPSILON);

        // Division by zero protection: target == initial
        let progress_zero = KeyResult::calculate_progress(50.0, 50.0, 50.0);
        assert_eq!(progress_zero, 100.0);

        let progress_zero_incomplete = KeyResult::calculate_progress(50.0, 50.0, 40.0);
        assert_eq!(progress_zero_incomplete, 0.0);
    }

    #[test]
    fn test_objective_weighted_progress_update() {
        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let mut obj = Objective::new(
            "Achieve Peak Physical Fitness",
            Some("Q4 OKR for marathon preparation".to_string()),
            "2026-Q4",
            start,
            end,
        );

        assert_eq!(obj.progress_percentage, 0.0);
        assert!(!obj.is_completed);

        // KR 1: Run 100km (weight 2.0), currently at 50km (50% progress)
        let mut kr1 = KeyResult::new_numeric(obj.id, "Run 100km", 0.0, 100.0, "km", 2.0);
        kr1.update_current_value(50.0);
        assert_eq!(kr1.progress_percentage, 50.0);

        // KR 2: 20 Workout sessions (weight 1.0), currently at 20 (100% progress)
        let mut kr2 = KeyResult::new_numeric(obj.id, "Gym sessions", 0.0, 20.0, "sessions", 1.0);
        kr2.update_current_value(20.0);
        assert_eq!(kr2.progress_percentage, 100.0);

        // Weighted Average = (50% * 2.0 + 100% * 1.0) / (2.0 + 1.0) = 200 / 3 = 66.66667%
        obj.update_progress(&[kr1.clone(), kr2.clone()]);
        assert!((obj.progress_percentage - 66.66667).abs() < 0.01);
        assert!(!obj.is_completed);

        // Complete KR 1 as well -> 100%
        kr1.update_current_value(100.0);
        obj.update_progress(&[kr1, kr2]);
        assert_eq!(obj.progress_percentage, 100.0);
        assert!(obj.is_completed);
    }

    #[tokio::test]
    async fn test_okr_sqlite_migrations_and_foreign_keys() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // 1. Insert Objective
        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new(
            "Learn Rust & Systems Programming",
            None,
            "2026-Q4",
            start,
            end,
        );

        sqlx::query(
            r"
            INSERT INTO objectives (id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(obj.id)
        .bind(&obj.title)
        .bind(&obj.description)
        .bind(&obj.quarter)
        .bind(obj.start_date)
        .bind(obj.end_date)
        .bind(obj.progress_percentage)
        .bind(obj.is_completed)
        .bind(obj.created_at)
        .bind(obj.updated_at)
        .execute(&pool)
        .await
        .unwrap();

        // 2. Insert Key Result
        let kr = KeyResult::new_numeric(obj.id, "Build 5 CLI tools", 0.0, 5.0, "tools", 1.0);
        sqlx::query(
            r"
            INSERT INTO key_results (id, objective_id, linked_habit_id, title, initial_value, target_value, current_value, unit, weight, progress_percentage, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(kr.id)
        .bind(kr.objective_id)
        .bind(kr.linked_habit_id)
        .bind(&kr.title)
        .bind(kr.initial_value)
        .bind(kr.target_value)
        .bind(kr.current_value)
        .bind(&kr.unit)
        .bind(kr.weight)
        .bind(kr.progress_percentage)
        .bind(kr.created_at)
        .bind(kr.updated_at)
        .execute(&pool)
        .await
        .unwrap();

        // 3. Query Objective with FromRow
        let fetched_obj = sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ?")
            .bind(obj.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched_obj.id, obj.id);
        assert_eq!(fetched_obj.title, obj.title);
        assert_eq!(fetched_obj.quarter, "2026-Q4");

        // 4. Query Key Result with FromRow
        let fetched_kr = sqlx::query_as::<_, KeyResult>("SELECT * FROM key_results WHERE id = ?")
            .bind(kr.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched_kr.id, kr.id);
        assert_eq!(fetched_kr.objective_id, obj.id);
        assert_eq!(fetched_kr.title, "Build 5 CLI tools");

        // 5. Test Cascade Deletion: Deleting Objective drops child Key Results
        sqlx::query("DELETE FROM objectives WHERE id = ?")
            .bind(obj.id)
            .execute(&pool)
            .await
            .unwrap();

        let remaining_krs =
            sqlx::query_as::<_, KeyResult>("SELECT * FROM key_results WHERE objective_id = ?")
                .bind(obj.id)
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(remaining_krs.len(), 0);
    }
}
