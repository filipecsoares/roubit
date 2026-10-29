//! Daily Routine Execution & Progress Tracker (RF01)
//!
//! Provides real-time calculation of routine position, current active block,
//! upcoming block countdown, and daily adherence percentage.

use std::collections::HashMap;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::{BlockStatus, TimeBlock};

/// Snapshot of the user's execution progress within their daily routine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentRoutineProgress {
    /// Total number of time blocks scheduled in the routine.
    pub total_blocks: usize,

    /// Number of blocks marked as [`BlockStatus::Completed`].
    pub completed_blocks: usize,

    /// Number of blocks marked as [`BlockStatus::Skipped`].
    pub skipped_blocks: usize,

    /// Number of blocks still pending or currently in progress.
    pub pending_blocks: usize,

    /// Percentage of completed blocks relative to total scheduled (0.0 to 100.0).
    pub adherence_percentage: f32,

    /// The time block currently underway at the evaluated time, if any.
    pub active_block: Option<TimeBlock>,

    /// The next upcoming time block scheduled after the evaluated time, if any.
    pub next_block: Option<TimeBlock>,

    /// Remaining minutes until the `next_block` starts, if any.
    pub minutes_until_next: Option<i64>,
}

/// Real-time evaluator for routine timeline progression and adherence metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutineProgressTracker;

impl RoutineProgressTracker {
    /// Evaluates the routine execution state and adherence metrics at a specified `current_time`.
    ///
    /// - **Adherence Rate:** `(completed_blocks / total_blocks) * 100.0`.
    ///   Safely evaluates to `0.0` when `total_blocks == 0`.
    /// - **Active Block:** Evaluates to the block where `start_time <= current_time < end_time`.
    /// - **Next Block:** Identifies the earliest block with `start_time > current_time`.
    /// - **Minutes Until Next:** Computes `(next_block.start_time - current_time).num_minutes()`.
    #[must_use]
    pub fn compute_progress(
        blocks: &[TimeBlock],
        block_statuses: &[(TimeBlock, BlockStatus)],
        current_time: NaiveTime,
    ) -> CurrentRoutineProgress {
        let total_blocks = blocks.len();

        let status_by_id: HashMap<Uuid, BlockStatus> =
            block_statuses.iter().map(|(b, s)| (b.id, *s)).collect();

        let mut completed_blocks = 0;
        let mut skipped_blocks = 0;
        let mut pending_blocks = 0;

        for block in blocks {
            match status_by_id
                .get(&block.id)
                .copied()
                .unwrap_or(BlockStatus::Pending)
            {
                BlockStatus::Completed => completed_blocks += 1,
                BlockStatus::Skipped => skipped_blocks += 1,
                BlockStatus::Pending | BlockStatus::InProgress => pending_blocks += 1,
            }
        }

        let adherence_percentage = if total_blocks == 0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let percentage = (completed_blocks as f32 / total_blocks as f32) * 100.0;
            percentage
        };

        let active_block = blocks
            .iter()
            .find(|b| b.start_time <= current_time && current_time < b.end_time)
            .cloned();

        let next_block = blocks
            .iter()
            .filter(|b| b.start_time > current_time)
            .min_by_key(|b| b.start_time)
            .cloned();

        let minutes_until_next = next_block
            .as_ref()
            .map(|b| (b.start_time - current_time).num_minutes());

        CurrentRoutineProgress {
            total_blocks,
            completed_blocks,
            skipped_blocks,
            pending_blocks,
            adherence_percentage,
            active_block,
            next_block,
            minutes_until_next,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    fn mock_block(name: &str, start: &str, end: &str) -> TimeBlock {
        TimeBlock::new(
            Uuid::new_v4(),
            name,
            NaiveTime::parse_from_str(start, "%H:%M:%S").expect("valid start time"),
            NaiveTime::parse_from_str(end, "%H:%M:%S").expect("valid end time"),
            0,
        )
    }

    #[test]
    fn test_routine_progress_calculation() {
        let b1 = mock_block("Morning", "07:00:00", "09:00:00");
        let b2 = mock_block("Focus", "09:30:00", "12:00:00");

        let progress = RoutineProgressTracker::compute_progress(
            &[b1.clone(), b2.clone()],
            &[(b1, BlockStatus::Completed), (b2, BlockStatus::Pending)],
            NaiveTime::from_hms_opt(10, 0, 0).expect("valid time"),
        );

        assert_eq!(progress.total_blocks, 2);
        assert_eq!(progress.completed_blocks, 1);
        assert_eq!(progress.pending_blocks, 1);
        assert_eq!(progress.skipped_blocks, 0);
        assert_eq!(progress.adherence_percentage, 50.0);
        assert_eq!(
            progress.active_block.as_ref().map(|b| b.name.as_str()),
            Some("Focus")
        );
        assert!(progress.next_block.is_none());
        assert!(progress.minutes_until_next.is_none());
    }

    #[test]
    fn test_empty_schedule_returns_zero_adherence_without_panic() {
        let progress = RoutineProgressTracker::compute_progress(
            &[],
            &[],
            NaiveTime::from_hms_opt(12, 0, 0).expect("valid time"),
        );

        assert_eq!(progress.total_blocks, 0);
        assert_eq!(progress.completed_blocks, 0);
        assert_eq!(progress.skipped_blocks, 0);
        assert_eq!(progress.pending_blocks, 0);
        assert_eq!(progress.adherence_percentage, 0.0);
        assert!(progress.active_block.is_none());
        assert!(progress.next_block.is_none());
        assert!(progress.minutes_until_next.is_none());
    }

    #[test]
    fn test_timeline_gap_detects_next_block_and_countdown() {
        let b1 = mock_block("Breakfast", "07:00:00", "08:00:00");
        let b2 = mock_block("Deep Work", "09:00:00", "11:00:00");

        // Evaluated at 08:15:00 (inside 1-hour gap between 08:00 and 09:00)
        let current_time = NaiveTime::from_hms_opt(8, 15, 0).expect("valid time");
        let progress = RoutineProgressTracker::compute_progress(
            &[b1.clone(), b2.clone()],
            &[(b1, BlockStatus::Completed), (b2, BlockStatus::Pending)],
            current_time,
        );

        assert!(progress.active_block.is_none());
        assert_eq!(
            progress.next_block.as_ref().map(|b| b.name.as_str()),
            Some("Deep Work")
        );
        assert_eq!(progress.minutes_until_next, Some(45));
    }

    #[test]
    fn test_before_day_starts_identifies_first_block() {
        let b1 = mock_block("Morning Workout", "06:30:00", "07:30:00");
        let b2 = mock_block("Review", "08:00:00", "09:00:00");

        // Evaluated at 05:30:00
        let current_time = NaiveTime::from_hms_opt(5, 30, 0).expect("valid time");
        let progress = RoutineProgressTracker::compute_progress(
            &[b1.clone(), b2.clone()],
            &[(b1, BlockStatus::Pending), (b2, BlockStatus::Pending)],
            current_time,
        );

        assert!(progress.active_block.is_none());
        assert_eq!(
            progress.next_block.as_ref().map(|b| b.name.as_str()),
            Some("Morning Workout")
        );
        assert_eq!(progress.minutes_until_next, Some(60));
        assert_eq!(progress.pending_blocks, 2);
    }

    #[test]
    fn test_after_day_ends_identifies_no_active_or_next_blocks() {
        let b1 = mock_block("Work", "09:00:00", "17:00:00");

        // Evaluated at 21:00:00
        let current_time = NaiveTime::from_hms_opt(21, 0, 0).expect("valid time");
        let progress = RoutineProgressTracker::compute_progress(
            std::slice::from_ref(&b1),
            &[(b1.clone(), BlockStatus::Completed)],
            current_time,
        );

        assert_eq!(progress.adherence_percentage, 100.0);
        assert!(progress.active_block.is_none());
        assert!(progress.next_block.is_none());
        assert!(progress.minutes_until_next.is_none());
    }

    #[test]
    fn test_exact_start_and_end_boundaries() {
        let b1 = mock_block("Block A", "08:00:00", "09:00:00");
        let b2 = mock_block("Block B", "09:00:00", "10:00:00");

        let blocks = [b1, b2];

        // Exact start of Block A
        let p_start = RoutineProgressTracker::compute_progress(
            &blocks,
            &[],
            NaiveTime::from_hms_opt(8, 0, 0).expect("valid time"),
        );
        assert_eq!(
            p_start.active_block.as_ref().map(|b| b.name.as_str()),
            Some("Block A")
        );

        // Exact transition: 09:00:00 is end of A and start of B
        let p_transition = RoutineProgressTracker::compute_progress(
            &blocks,
            &[],
            NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"),
        );
        assert_eq!(
            p_transition.active_block.as_ref().map(|b| b.name.as_str()),
            Some("Block B")
        );
    }

    #[test]
    fn test_skipped_and_in_progress_status_handling() {
        let b1 = mock_block("Task 1", "08:00:00", "09:00:00");
        let b2 = mock_block("Task 2", "09:00:00", "10:00:00");
        let b3 = mock_block("Task 3", "10:00:00", "11:00:00");
        let b4 = mock_block("Task 4", "11:00:00", "12:00:00");

        let progress = RoutineProgressTracker::compute_progress(
            &[b1.clone(), b2.clone(), b3.clone(), b4.clone()],
            &[
                (b1, BlockStatus::Completed),
                (b2, BlockStatus::Skipped),
                (b3, BlockStatus::InProgress),
                (b4, BlockStatus::Pending),
            ],
            NaiveTime::from_hms_opt(10, 30, 0).expect("valid time"),
        );

        assert_eq!(progress.total_blocks, 4);
        assert_eq!(progress.completed_blocks, 1);
        assert_eq!(progress.skipped_blocks, 1);
        assert_eq!(progress.pending_blocks, 2); // InProgress + Pending
        assert_eq!(progress.adherence_percentage, 25.0);
        assert_eq!(
            progress.active_block.as_ref().map(|b| b.name.as_str()),
            Some("Task 3")
        );
        assert_eq!(
            progress.next_block.as_ref().map(|b| b.name.as_str()),
            Some("Task 4")
        );
        assert_eq!(progress.minutes_until_next, Some(30));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let b1 = mock_block("Sprint", "09:00:00", "10:00:00");
        let progress = RoutineProgressTracker::compute_progress(
            std::slice::from_ref(&b1),
            &[(b1.clone(), BlockStatus::Completed)],
            NaiveTime::from_hms_opt(9, 30, 0).expect("valid time"),
        );

        let json = serde_json::to_string(&progress).expect("serialization succeeds");
        let deserialized: CurrentRoutineProgress =
            serde_json::from_str(&json).expect("deserialization succeeds");

        assert_eq!(progress, deserialized);
    }
}
