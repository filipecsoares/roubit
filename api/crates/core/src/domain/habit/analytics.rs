use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::models::{Habit, HabitLog};
use super::streak::StreakCalculator;

/// A single cell in the habit contribution heatmap grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatmapCell {
    /// Calendar date of the cell.
    pub date: NaiveDate,
    /// Total recorded value for this date.
    pub count: f64,
    /// Target value of the habit for this date.
    pub target: f64,
    /// Intensity bucket from 0 to 4 (GitHub contributions style).
    pub intensity: u8,
}

/// Aggregated consistency and adherence statistics for a habit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitAnalyticsSummary {
    /// Total number of check-in log records available.
    pub total_logs_count: usize,
    /// Adherence percentage in the last 30 days (0.0 to 100.0).
    pub completion_rate_30d: f32,
    /// Adherence percentage in the last 90 days (0.0 to 100.0).
    pub completion_rate_90d: f32,
    /// Adherence percentage across all recorded days (0.0 to 100.0).
    pub all_time_completion_rate: f32,
    /// Continuous grid of heatmap cells over the evaluated period.
    pub heatmap_cells: Vec<HeatmapCell>,
}

/// Service providing heatmap matrix generation and adherence rate analytics.
#[derive(Debug)]
pub struct HabitAnalyticsService;

impl HabitAnalyticsService {
    /// Calculates the intensity level (0 to 4) based on actual vs target value.
    ///
    /// - Level 0: 0% completion (or count <= 0.0)
    /// - Level 1: 1% - 39% completion
    /// - Level 2: 40% - 79% completion
    /// - Level 3: 80% - 99% completion
    /// - Level 4: 100%+ completion (target fully achieved or exceeded)
    #[must_use]
    pub fn calculate_intensity(actual_value: f64, target_value: f64) -> u8 {
        if actual_value <= 0.0 || target_value <= 0.0 {
            0
        } else {
            let ratio = actual_value / target_value;
            if ratio < 0.40 {
                1
            } else if ratio < 0.80 {
                2
            } else if ratio < 1.00 {
                3
            } else {
                4
            }
        }
    }

    /// Generates a continuous sequence of heatmap cells between `start_date` and `end_date`,
    /// filling missing dates with Level 0 intensity so the resulting vector forms an uninterrupted calendar grid.
    #[must_use]
    pub fn generate_heatmap(
        logs: &[(NaiveDate, f64)],
        target_value: f64,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<HeatmapCell> {
        if start_date > end_date {
            return Vec::new();
        }

        let mut log_map: HashMap<NaiveDate, f64> = HashMap::with_capacity(logs.len());
        for &(date, val) in logs {
            *log_map.entry(date).or_insert(0.0) += val;
        }

        let mut cells = Vec::new();
        let mut curr = start_date;

        while curr <= end_date {
            let count = log_map.get(&curr).copied().unwrap_or(0.0);
            let intensity = Self::calculate_intensity(count, target_value);

            cells.push(HeatmapCell {
                date: curr,
                count,
                target: target_value,
                intensity,
            });

            match curr.succ_opt() {
                Some(next) => curr = next,
                None => break,
            }
        }

        cells
    }

    /// Calculates adherence percentage given qualifying completed days vs required scheduled days.
    /// Returns 0.0 to 100.0, guarded against division-by-zero.
    #[must_use]
    pub fn calculate_window_adherence(completed_days: usize, required_days: usize) -> f32 {
        if required_days == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let rate = (completed_days as f32 / required_days as f32) * 100.0;
        rate.clamp(0.0, 100.0)
    }

    /// Computes adherence percentage for a habit in a specified date range `[start_date, end_date]`.
    #[must_use]
    pub fn compute_adherence_in_range(
        habit: &Habit,
        logs: &[HabitLog],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> f32 {
        if start_date > end_date || habit.frequency_days == 0 {
            return 0.0;
        }

        let completed_set: HashSet<NaiveDate> = logs
            .iter()
            .filter(|l| l.is_completed && l.log_date >= start_date && l.log_date <= end_date)
            .map(|l| l.log_date)
            .collect();

        let mut required_days: usize = 0;
        let mut completed_days: usize = 0;
        let mut curr = start_date;

        while curr <= end_date {
            if StreakCalculator::is_required_day(curr, habit.frequency_days) {
                required_days += 1;
                if completed_set.contains(&curr) {
                    completed_days += 1;
                }
            }

            match curr.succ_opt() {
                Some(next) => curr = next,
                None => break,
            }
        }

        Self::calculate_window_adherence(completed_days, required_days)
    }

    /// Computes a complete analytics summary for a habit, including 30-day, 90-day, and all-time adherence,
    /// along with an uninterrupted annual heatmap matrix (up to 365 days).
    #[must_use]
    pub fn compute_summary(
        habit: &Habit,
        logs: &[HabitLog],
        reference_date: NaiveDate,
    ) -> HabitAnalyticsSummary {
        // 30-day window: [reference_date - 29 days, reference_date]
        let start_30d = reference_date - chrono::Duration::days(29);
        let completion_rate_30d =
            Self::compute_adherence_in_range(habit, logs, start_30d, reference_date);

        // 90-day window: [reference_date - 89 days, reference_date]
        let start_90d = reference_date - chrono::Duration::days(89);
        let completion_rate_90d =
            Self::compute_adherence_in_range(habit, logs, start_90d, reference_date);

        // All-time window: from earliest log or creation date to reference_date
        let earliest_log_date = logs.iter().map(|l| l.log_date).min();
        let creation_date = habit.created_at.date_naive();
        let start_all_time = earliest_log_date
            .map_or(creation_date, |d| d.min(creation_date))
            .min(reference_date);

        let all_time_completion_rate =
            Self::compute_adherence_in_range(habit, logs, start_all_time, reference_date);

        // Heatmap for the past 365 days
        let start_heatmap = reference_date - chrono::Duration::days(364);
        let log_tuples: Vec<(NaiveDate, f64)> = logs
            .iter()
            .filter(|l| l.log_date >= start_heatmap && l.log_date <= reference_date)
            .map(|l| (l.log_date, l.actual_value))
            .collect();

        let heatmap_cells = Self::generate_heatmap(
            &log_tuples,
            habit.target_value,
            start_heatmap,
            reference_date,
        );

        HabitAnalyticsSummary {
            total_logs_count: logs.len(),
            completion_rate_30d,
            completion_rate_90d,
            all_time_completion_rate,
            heatmap_cells,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_heatmap_intensity_levels() {
        let target = 100.0;
        let logs = vec![
            (NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(), 0.0), // Level 0
            (NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(), 50.0), // Level 2
            (NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(), 100.0), // Level 4
        ];

        let cells = HabitAnalyticsService::generate_heatmap(
            &logs,
            target,
            NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(),
        );

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].intensity, 0);
        assert_eq!(cells[1].intensity, 2);
        assert_eq!(cells[2].intensity, 4);
    }

    #[test]
    fn test_heatmap_all_intensity_thresholds() {
        let target = 100.0;
        assert_eq!(HabitAnalyticsService::calculate_intensity(0.0, target), 0);
        assert_eq!(HabitAnalyticsService::calculate_intensity(-5.0, target), 0);
        assert_eq!(HabitAnalyticsService::calculate_intensity(20.0, target), 1); // 20% -> 1
        assert_eq!(HabitAnalyticsService::calculate_intensity(39.9, target), 1); // 39.9% -> 1
        assert_eq!(HabitAnalyticsService::calculate_intensity(40.0, target), 2); // 40% -> 2
        assert_eq!(HabitAnalyticsService::calculate_intensity(79.9, target), 2); // 79.9% -> 2
        assert_eq!(HabitAnalyticsService::calculate_intensity(80.0, target), 3); // 80% -> 3
        assert_eq!(HabitAnalyticsService::calculate_intensity(99.9, target), 3); // 99.9% -> 3
        assert_eq!(HabitAnalyticsService::calculate_intensity(100.0, target), 4); // 100% -> 4
        assert_eq!(HabitAnalyticsService::calculate_intensity(250.0, target), 4);
        // >100% capped at 4
    }

    #[test]
    fn test_heatmap_fills_uninterrupted_calendar_range() {
        let target = 1000.0;
        let start = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        // Only one log on the 27th
        let logs = vec![(NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(), 1000.0)];

        let cells = HabitAnalyticsService::generate_heatmap(&logs, target, start, end);
        assert_eq!(cells.len(), 5); // 25, 26, 27, 28, 29
        assert_eq!(cells[0].intensity, 0);
        assert_eq!(cells[1].intensity, 0);
        assert_eq!(cells[2].intensity, 4);
        assert_eq!(cells[3].intensity, 0);
        assert_eq!(cells[4].intensity, 0);
    }

    #[test]
    fn test_calculate_window_adherence_edge_cases() {
        assert_eq!(HabitAnalyticsService::calculate_window_adherence(0, 0), 0.0);
        assert_eq!(
            HabitAnalyticsService::calculate_window_adherence(5, 10),
            50.0
        );
        assert_eq!(
            HabitAnalyticsService::calculate_window_adherence(10, 10),
            100.0
        );
        assert_eq!(
            HabitAnalyticsService::calculate_window_adherence(15, 10),
            100.0
        ); // clamped
    }

    #[test]
    fn test_compute_summary_30d_and_90d() {
        use uuid::Uuid;

        let habit = Habit::new_numeric("Drink Water", None, 2000.0, "ml");
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        // 15 completed logs in the last 30 days
        let mut logs = Vec::new();
        for i in 0..15 {
            logs.push(HabitLog {
                id: Uuid::new_v4(),
                habit_id: habit.id,
                log_date: today - chrono::Duration::days(i),
                actual_value: 2000.0,
                is_completed: true,
                notes: None,
                logged_at: chrono::Utc::now(),
            });
        }

        let summary = HabitAnalyticsService::compute_summary(&habit, &logs, today);
        assert_eq!(summary.total_logs_count, 15);
        // 15 days completed out of 30 days scheduled = 50.0%
        assert!((summary.completion_rate_30d - 50.0).abs() < f32::EPSILON);
        assert_eq!(summary.heatmap_cells.len(), 365);
    }
}
