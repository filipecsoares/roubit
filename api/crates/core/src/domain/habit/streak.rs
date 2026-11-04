use std::collections::HashSet;

use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use super::models::{Habit, HabitLog};

/// Streak calculation metrics for a habit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreakMetrics {
    /// Number of consecutive qualifying days completed in the current active streak.
    pub current_streak: u32,
    /// Maximum number of consecutive qualifying days completed all-time.
    pub longest_streak: u32,
    /// Whether the habit was completed on the reference date (e.g. today).
    pub is_active_today: bool,
}

/// Resilient calculation engine for habit streaks.
#[derive(Debug)]
pub struct StreakCalculator;

impl StreakCalculator {
    /// Computes whether a given date is required according to the 7-day frequency bitmask.
    /// Bit 0 = Monday, Bit 1 = Tuesday, ..., Bit 6 = Sunday.
    #[must_use]
    pub fn is_required_day(date: NaiveDate, frequency_bitmask: u8) -> bool {
        let bit = match date.weekday() {
            Weekday::Mon => 1 << 0,
            Weekday::Tue => 1 << 1,
            Weekday::Wed => 1 << 2,
            Weekday::Thu => 1 << 3,
            Weekday::Fri => 1 << 4,
            Weekday::Sat => 1 << 5,
            Weekday::Sun => 1 << 6,
        };
        (frequency_bitmask & bit) != 0
    }

    /// Incrementally computes the updated `(current_streak, longest_streak)` after a completion event.
    ///
    /// # Arguments
    /// * `current_streak` - Existing consecutive completion count on the habit.
    /// * `longest_streak` - All-time longest streak recorded on the habit.
    /// * `last_checkin_date` - Most recent date of completion strictly prior to `new_checkin_date`, if any.
    /// * `new_checkin_date` - Calendar date of the newly completed check-in.
    /// * `frequency_bitmask` - 7-day scheduled frequency bitmask.
    ///
    /// # Returns
    /// A tuple `(new_current_streak, new_longest_streak)` computed in O(1) steady-state.
    #[must_use]
    #[allow(clippy::option_if_let_else)]
    pub fn update_streak_incremental(
        current_streak: i32,
        longest_streak: i32,
        last_checkin_date: Option<NaiveDate>,
        new_checkin_date: NaiveDate,
        frequency_bitmask: u8,
    ) -> (i32, i32) {
        if frequency_bitmask == 0 {
            return (0, 0);
        }

        let is_required = Self::is_required_day(new_checkin_date, frequency_bitmask);

        let new_current = match last_checkin_date {
            None => i32::from(is_required),
            Some(prev_date) => match new_checkin_date.cmp(&prev_date) {
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => current_streak,
                std::cmp::Ordering::Greater => {
                    let mut missed_required_day = false;
                    let mut check_date = prev_date.succ_opt();
                    while let Some(d) = check_date {
                        if d >= new_checkin_date {
                            break;
                        }
                        if Self::is_required_day(d, frequency_bitmask) {
                            missed_required_day = true;
                            break;
                        }
                        check_date = d.succ_opt();
                    }

                    if missed_required_day {
                        i32::from(is_required)
                    } else if is_required {
                        current_streak + 1
                    } else {
                        current_streak
                    }
                }
            },
        };

        let new_longest = longest_streak.max(new_current);
        (new_current, new_longest)
    }

    /// Evaluates whether a denormalized `current_streak` is still active on `reference_date`
    /// given the most recent completed log date (`last_completed_date <= reference_date`)
    /// and the habit's 7-day `frequency_bitmask`.
    ///
    /// If any scheduled required day strictly between `last_completed_date` and `reference_date`
    /// was missed, the streak has broken and decays to `0`.
    #[must_use]
    pub fn effective_current_streak(
        stored_current_streak: i32,
        last_completed_date: Option<NaiveDate>,
        reference_date: NaiveDate,
        frequency_bitmask: u8,
    ) -> u32 {
        if stored_current_streak <= 0 || frequency_bitmask == 0 {
            return 0;
        }

        let Some(last_date) = last_completed_date else {
            return 0;
        };

        if last_date > reference_date {
            return u32::try_from(stored_current_streak).unwrap_or(0);
        }

        let mut check_date = last_date.succ_opt();
        while let Some(d) = check_date {
            if d >= reference_date {
                break;
            }
            if Self::is_required_day(d, frequency_bitmask) {
                return 0;
            }
            check_date = d.succ_opt();
        }

        u32::try_from(stored_current_streak).unwrap_or(0)
    }

    /// Computes current and longest streak given a list of completed dates, habit frequency,
    /// and the reference date (typically today).
    #[must_use]
    pub fn calculate(
        completed_dates: &[NaiveDate],
        frequency_bitmask: u8,
        reference_date: NaiveDate,
    ) -> StreakMetrics {
        if frequency_bitmask == 0 {
            return StreakMetrics {
                current_streak: 0,
                longest_streak: 0,
                is_active_today: false,
            };
        }

        // Deduplicate and filter out any anomalous future dates beyond reference_date
        let completed_set: HashSet<NaiveDate> = completed_dates
            .iter()
            .copied()
            .filter(|&d| d <= reference_date)
            .collect();

        let is_active_today = completed_set.contains(&reference_date);

        if completed_set.is_empty() {
            return StreakMetrics {
                current_streak: 0,
                longest_streak: 0,
                is_active_today,
            };
        }

        // 1. Calculate Current Streak (working backwards from reference_date)
        // If reference_date is completed and is a required day, it starts at 1.
        let mut current_streak: u32 =
            u32::from(is_active_today && Self::is_required_day(reference_date, frequency_bitmask));

        let min_completed_date = *completed_set.iter().min().unwrap_or(&reference_date);

        // Scan backwards starting from reference_date - 1 day
        if let Some(mut curr) = reference_date.pred_opt() {
            while curr >= min_completed_date {
                if Self::is_required_day(curr, frequency_bitmask) {
                    if completed_set.contains(&curr) {
                        current_streak += 1;
                    } else {
                        // Uncompleted required day breaks the current streak
                        break;
                    }
                }
                // Non-required days (e.g. weekends on weekday habits) are skipped without breaking streak
                match curr.pred_opt() {
                    Some(prev) => curr = prev,
                    None => break,
                }
            }
        }

        // 2. Calculate Longest Streak (scan chronologically from min_completed_date up to reference_date)
        let mut longest_streak: u32 = 0;
        let mut current_run: u32 = 0;
        let mut curr = min_completed_date;

        while curr <= reference_date {
            if Self::is_required_day(curr, frequency_bitmask) {
                if completed_set.contains(&curr) {
                    current_run += 1;
                    if current_run > longest_streak {
                        longest_streak = current_run;
                    }
                } else {
                    current_run = 0;
                }
            }
            // Non-required days do not reset or increment current_run

            match curr.succ_opt() {
                Some(next) => curr = next,
                None => break,
            }
        }

        // Longest streak must always be at least current_streak
        longest_streak = longest_streak.max(current_streak);

        StreakMetrics {
            current_streak,
            longest_streak,
            is_active_today,
        }
    }

    /// Convenience helper to calculate streak metrics for a [`Habit`] and its [`HabitLog`] records.
    #[must_use]
    pub fn calculate_for_habit(
        habit: &Habit,
        logs: &[HabitLog],
        reference_date: NaiveDate,
    ) -> StreakMetrics {
        let completed_dates: Vec<NaiveDate> = logs
            .iter()
            .filter(|log| log.is_completed)
            .map(|log| log.log_date)
            .collect();
        Self::calculate(&completed_dates, habit.frequency_days, reference_date)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_streak_unbroken() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let dates = vec![
            today,
            today - chrono::Duration::days(1),
            today - chrono::Duration::days(2),
            today - chrono::Duration::days(3),
        ];

        let metrics = StreakCalculator::calculate(&dates, 127, today);
        assert_eq!(metrics.current_streak, 4);
        assert_eq!(metrics.longest_streak, 4);
        assert!(metrics.is_active_today);
    }

    #[test]
    fn test_weekday_streak_skips_weekends() {
        // Mon Oct 26 to Fri Oct 30
        let friday = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let wednesday = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();

        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(); // Following Monday
        let dates = vec![monday, friday, thursday, wednesday];

        // Bitmask for Mon-Fri: 1+2+4+8+16 = 31
        let metrics = StreakCalculator::calculate(&dates, 31, monday);
        assert_eq!(metrics.current_streak, 4); // Sat & Sun did not reset streak!
        assert_eq!(metrics.longest_streak, 4);
        assert!(metrics.is_active_today);
    }

    #[test]
    fn test_grace_period_preserves_streak_when_today_is_pending() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(); // Thursday
        let yesterday = today - chrono::Duration::days(1);
        let day_before = today - chrono::Duration::days(2);

        // Today is not completed yet!
        let dates = vec![yesterday, day_before];

        let metrics = StreakCalculator::calculate(&dates, 127, today);
        assert_eq!(metrics.current_streak, 2);
        assert_eq!(metrics.longest_streak, 2);
        assert!(!metrics.is_active_today);
    }

    #[test]
    fn test_broken_streak_resets_current_to_zero_but_retains_longest() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        // Completed 5 consecutive days, then missed 2 days, then missed today
        let dates = vec![
            today - chrono::Duration::days(3),
            today - chrono::Duration::days(4),
            today - chrono::Duration::days(5),
            today - chrono::Duration::days(6),
            today - chrono::Duration::days(7),
        ];

        let metrics = StreakCalculator::calculate(&dates, 127, today);
        assert_eq!(metrics.current_streak, 0);
        assert_eq!(metrics.longest_streak, 5);
        assert!(!metrics.is_active_today);
    }

    #[test]
    fn test_empty_logs_returns_zeroes() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let metrics = StreakCalculator::calculate(&[], 127, today);
        assert_eq!(metrics.current_streak, 0);
        assert_eq!(metrics.longest_streak, 0);
        assert!(!metrics.is_active_today);
    }

    #[test]
    fn test_zero_frequency_bitmask_returns_zeroes() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let dates = vec![today, today - chrono::Duration::days(1)];
        let metrics = StreakCalculator::calculate(&dates, 0, today);
        assert_eq!(metrics.current_streak, 0);
        assert_eq!(metrics.longest_streak, 0);
        assert!(!metrics.is_active_today);
    }

    #[test]
    fn test_future_dates_are_ignored() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let tomorrow = today + chrono::Duration::days(1);
        let dates = vec![today, tomorrow];

        let metrics = StreakCalculator::calculate(&dates, 127, today);
        assert_eq!(metrics.current_streak, 1);
        assert_eq!(metrics.longest_streak, 1);
        assert!(metrics.is_active_today);
    }

    #[test]
    fn test_duplicate_dates_are_deduplicated() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let yesterday = today - chrono::Duration::days(1);
        let dates = vec![today, today, yesterday, yesterday, yesterday];

        let metrics = StreakCalculator::calculate(&dates, 127, today);
        assert_eq!(metrics.current_streak, 2);
        assert_eq!(metrics.longest_streak, 2);
    }

    #[test]
    fn test_calculate_for_habit_helper() {
        use uuid::Uuid;

        let habit = Habit::new_numeric("Read Book", None, 20.0, "pages");

        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let logs = vec![
            HabitLog {
                id: Uuid::new_v4(),
                habit_id: habit.id,
                log_date: today,
                actual_value: 25.0,
                is_completed: true,
                notes: None,
                logged_at: chrono::Utc::now(),
            },
            HabitLog {
                id: Uuid::new_v4(),
                habit_id: habit.id,
                log_date: today - chrono::Duration::days(1),
                actual_value: 10.0,
                is_completed: false, // Target 20.0 not reached!
                notes: None,
                logged_at: chrono::Utc::now(),
            },
        ];

        let metrics = StreakCalculator::calculate_for_habit(&habit, &logs, today);
        assert_eq!(metrics.current_streak, 1);
        assert_eq!(metrics.longest_streak, 1);
        assert!(metrics.is_active_today);
    }

    #[test]
    fn test_update_streak_incremental_first_checkin() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(); // Monday
        let (current, longest) =
            StreakCalculator::update_streak_incremental(0, 0, None, monday, 127);
        assert_eq!(current, 1);
        assert_eq!(longest, 1);
    }

    #[test]
    fn test_update_streak_incremental_same_day_reentry() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let (current, longest) =
            StreakCalculator::update_streak_incremental(3, 5, Some(monday), monday, 127);
        assert_eq!(current, 3);
        assert_eq!(longest, 5);
    }

    #[test]
    fn test_update_streak_incremental_daily_consecutive() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let tuesday = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
        let (current, longest) =
            StreakCalculator::update_streak_incremental(2, 4, Some(monday), tuesday, 127);
        assert_eq!(current, 3);
        assert_eq!(longest, 4);

        // Surpassing longest streak
        let wednesday = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        let (current, longest) =
            StreakCalculator::update_streak_incremental(4, 4, Some(tuesday), wednesday, 127);
        assert_eq!(current, 5);
        assert_eq!(longest, 5);
    }

    #[test]
    fn test_update_streak_incremental_weekday_skips_weekends() {
        let friday = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(); // Friday
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(); // Monday
                                                                    // Frequency bitmask 31 = Monday-Friday
        let (current, longest) =
            StreakCalculator::update_streak_incremental(5, 10, Some(friday), monday, 31);
        assert_eq!(current, 6);
        assert_eq!(longest, 10);
    }

    #[test]
    fn test_update_streak_incremental_broken_streak_gap() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let wednesday = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap(); // Missed Tuesday
        let (current, longest) =
            StreakCalculator::update_streak_incremental(10, 15, Some(monday), wednesday, 127);
        assert_eq!(current, 1);
        assert_eq!(longest, 15);
    }
}
