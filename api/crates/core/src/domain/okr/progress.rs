//! Habit-to-KR linkage and progress computation engine (RF03).

use uuid::Uuid;

use crate::domain::habit::{HabitRepository, HabitType};

use super::models::KeyResult;
use super::repository::{OkrRepository, OkrRepositoryError};

/// Pure calculation engine for Key Result and Objective progress percentages.
#[derive(Debug)]
pub struct OkrProgressEngine;

impl OkrProgressEngine {
    /// Pure function calculating progress percentage from initial, target, and current values.
    /// Safely handles division by zero, inverse/decreasing targets (e.g. weight reduction),
    /// and clamps output to `[0.0, 100.0]`.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_kr_progress(initial: f64, target: f64, current: f64) -> f32 {
        let delta = target - initial;
        if delta.abs() < f64::EPSILON {
            if current >= target {
                100.0
            } else {
                0.0
            }
        } else {
            let raw_progress = ((current - initial) / delta * 100.0) as f32;
            raw_progress.clamp(0.0, 100.0)
        }
    }

    /// Computes the weighted average progress percentage across child Key Results.
    /// If total weight <= 0.0 or list is empty, returns 0.0.
    #[must_use]
    #[allow(clippy::suboptimal_flops)]
    pub fn calculate_objective_progress(key_results: &[KeyResult]) -> f32 {
        if key_results.is_empty() {
            return 0.0;
        }

        let mut total_weight = 0.0f32;
        let mut weighted_sum = 0.0f32;

        for kr in key_results {
            let weight = if kr.weight <= 0.0 { 1.0 } else { kr.weight };
            total_weight += weight;
            weighted_sum += kr.progress_percentage * weight;
        }

        if total_weight > 0.0 {
            (weighted_sum / total_weight).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
}

/// Service synchronizing habit completion and quantitative metric check-ins to linked Key Results.
#[derive(Debug, Clone)]
pub struct OkrSyncService<O: OkrRepository, H: HabitRepository> {
    okr_repo: O,
    habit_repo: H,
}

impl<O, H> OkrSyncService<O, H>
where
    O: OkrRepository,
    H: HabitRepository,
{
    /// Creates a new `OkrSyncService` pairing an OKR repository with a Habit repository.
    #[must_use]
    pub const fn new(okr_repo: O, habit_repo: H) -> Self {
        Self {
            okr_repo,
            habit_repo,
        }
    }

    /// Returns a reference to the underlying OKR repository.
    #[must_use]
    pub const fn okr_repo(&self) -> &O {
        &self.okr_repo
    }

    /// Returns a reference to the underlying Habit repository.
    #[must_use]
    pub const fn habit_repo(&self) -> &H {
        &self.habit_repo
    }

    /// Finds all Key Results linked to `habit_id`, aggregates all qualifying habit logs
    /// within the parent Objective's time window, and updates the Key Result and Objective progress.
    ///
    /// # Errors
    /// Returns [`OkrRepositoryError`] if querying or updating objectives, key results, or habit logs fails.
    #[allow(clippy::option_if_let_else)]
    pub async fn sync_linked_krs_for_habit(
        &self,
        habit_id: Uuid,
    ) -> Result<(), OkrRepositoryError> {
        let linked_krs = self.okr_repo.list_key_results_by_habit(habit_id).await?;
        if linked_krs.is_empty() {
            return Ok(());
        }

        let habit_opt = self
            .habit_repo
            .get_habit_by_id(habit_id)
            .await
            .map_err(|e| OkrRepositoryError::Habit(e.to_string()))?;

        for kr in linked_krs {
            // Find parent Objective to determine time window
            let Some(parent) = self
                .okr_repo
                .get_objective_with_krs(kr.objective_id)
                .await?
            else {
                continue;
            };

            let logs = self
                .habit_repo
                .get_logs_for_habit(
                    habit_id,
                    parent.objective.start_date,
                    parent.objective.end_date,
                )
                .await
                .map_err(|e| OkrRepositoryError::Habit(e.to_string()))?;

            let new_value: f64 = if let Some(ref habit) = habit_opt {
                match habit.habit_type {
                    HabitType::Binary => {
                        #[allow(clippy::cast_precision_loss)]
                        let count = logs.iter().filter(|l| l.is_completed).count() as f64;
                        count
                    }
                    HabitType::Numeric | HabitType::Timer => {
                        logs.iter().map(|l| l.actual_value).sum()
                    }
                }
            } else {
                logs.iter().map(|l| l.actual_value).sum()
            };

            self.okr_repo
                .update_key_result_value(kr.id, new_value)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::similar_names)]
mod tests {
    use super::*;
    use crate::domain::habit::{Habit, SqliteHabitRepository};
    use crate::domain::okr::models::Objective;
    use crate::domain::okr::repository::SqliteOkrRepository;
    use chrono::NaiveDate;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_weighted_objective_progress() {
        let obj_id = Uuid::new_v4();
        let mut kr1 = KeyResult::new_numeric(obj_id, "KR 1", 0.0, 100.0, "units", 1.0);
        kr1.progress_percentage = 50.0;

        let mut kr2 = KeyResult::new_numeric(obj_id, "KR 2", 0.0, 100.0, "units", 3.0);
        kr2.progress_percentage = 100.0;

        // Weighted average: (50*1 + 100*3) / (1 + 3) = 350 / 4 = 87.5%
        let progress = OkrProgressEngine::calculate_objective_progress(&[kr1, kr2]);
        assert_eq!(progress, 87.5);
    }

    #[test]
    fn test_kr_progress_inversion_and_edge_cases() {
        // Normal progression: 0 to 100km, currently at 45km -> 45.0%
        let progress = OkrProgressEngine::calculate_kr_progress(0.0, 100.0, 45.0);
        assert!((progress - 45.0).abs() < f32::EPSILON);

        // Target exceeded: 0 to 100km, currently at 120km -> capped at 100.0%
        let progress_capped = OkrProgressEngine::calculate_kr_progress(0.0, 100.0, 120.0);
        assert_eq!(progress_capped, 100.0);

        // Inverse progression (e.g. weight loss from 90kg down to 80kg):
        // initial: 90, target: 80, current: 85 -> delta is -10, current-initial is -5 -> 50%
        let progress_desc = OkrProgressEngine::calculate_kr_progress(90.0, 80.0, 85.0);
        assert!((progress_desc - 50.0).abs() < f32::EPSILON);

        // Target equals initial
        let progress_equal = OkrProgressEngine::calculate_kr_progress(50.0, 50.0, 50.0);
        assert_eq!(progress_equal, 100.0);

        let progress_equal_incomplete = OkrProgressEngine::calculate_kr_progress(50.0, 50.0, 49.0);
        assert_eq!(progress_equal_incomplete, 0.0);
    }

    #[tokio::test]
    async fn test_sync_linked_krs_for_habit() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let okr_repo = SqliteOkrRepository::new(pool.clone());
        let habit_repo = SqliteHabitRepository::new(pool);
        let sync_service = OkrSyncService::new(okr_repo.clone(), habit_repo.clone());

        // 1. Create a Numeric Habit: Running (5km per run target)
        let habit = Habit::new_numeric("Running", None, 5.0, "km");
        habit_repo.create_habit(&habit).await.unwrap();

        // 2. Create an Objective: Run 100km in Q4
        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Cardio Mastery", None, "2026-Q4", start, end);
        okr_repo.create_objective(&obj).await.unwrap();

        // 3. Create a Key Result linked to the Running Habit
        let kr = KeyResult::new_habit_linked(obj.id, habit.id, "Log 100km Total", 100.0, "km", 1.0);
        okr_repo.create_key_result(&kr).await.unwrap();

        // 4. Record habit check-ins:
        // Day 1: 10km
        // Day 2: 15km
        let day1 = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        habit_repo
            .record_check_in(habit.id, day1, 10.0, true)
            .await
            .unwrap();
        habit_repo
            .record_check_in(habit.id, day2, 15.0, true)
            .await
            .unwrap();

        // 5. Trigger sync
        sync_service
            .sync_linked_krs_for_habit(habit.id)
            .await
            .unwrap();

        // 6. Verify Key Result and parent Objective are updated
        let loaded = okr_repo
            .get_objective_with_krs(obj.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.key_results.len(), 1);
        let updated_kr = &loaded.key_results[0];
        // 10km + 15km = 25km
        assert_eq!(updated_kr.current_value, 25.0);
        assert_eq!(updated_kr.progress_percentage, 25.0);

        // Parent Objective progress = 25.0%
        assert_eq!(loaded.objective.progress_percentage, 25.0);
        assert!(!loaded.objective.is_completed);
    }
}
