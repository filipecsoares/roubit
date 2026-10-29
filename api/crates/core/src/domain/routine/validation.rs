//! Time Block Validation & Overlap Engine (RF01)
//!
//! Provides domain validation for routine time blocks:
//! - Chronological boundaries (`start_time < end_time`)
//! - Minimum duration (at least 5 minutes)
//! - Schedule overlap detection across time blocks

use chrono::NaiveTime;
use thiserror::Error;

use super::models::TimeBlock;

/// Minimum allowed duration for a routine time block in minutes.
pub const MINIMUM_BLOCK_DURATION_MINUTES: i64 = 5;

/// Errors occurring during time block validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimeBlockValidationError {
    /// The start time does not precede the end time.
    #[error("Start time ({start}) must be before end time ({end})")]
    InvalidInterval {
        /// Start time of the block.
        start: NaiveTime,
        /// End time of the block.
        end: NaiveTime,
    },

    /// The block duration is below the minimum threshold of 5 minutes.
    #[error("Block duration ({minutes} mins) is below minimum of 5 minutes")]
    DurationTooShort {
        /// Computed duration in minutes.
        minutes: i64,
    },

    /// Overlap detected between two time blocks.
    #[error("Overlap detected between '{block_a}' ({start_a}-{end_a}) and '{block_b}' ({start_b}-{end_b})")]
    OverlapDetected {
        /// Name of the earlier time block.
        block_a: String,
        /// Start time of the earlier block.
        start_a: NaiveTime,
        /// End time of the earlier block.
        end_a: NaiveTime,
        /// Name of the overlapping time block.
        block_b: String,
        /// Start time of the overlapping block.
        start_b: NaiveTime,
        /// End time of the overlapping block.
        end_b: NaiveTime,
    },
}

/// Validates time blocks and verifies schedule non-overlap invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeBlockValidator;

impl TimeBlockValidator {
    /// Validates an individual [`TimeBlock`]:
    /// - `start_time` must precede `end_time`
    /// - Duration must be at least 5 minutes
    ///
    /// # Errors
    /// Returns [`TimeBlockValidationError::InvalidInterval`] if `start_time >= end_time`.
    /// Returns [`TimeBlockValidationError::DurationTooShort`] if duration is under 5 minutes.
    pub fn validate_single(block: &TimeBlock) -> Result<(), TimeBlockValidationError> {
        if block.start_time >= block.end_time {
            return Err(TimeBlockValidationError::InvalidInterval {
                start: block.start_time,
                end: block.end_time,
            });
        }

        let minutes = block.duration_minutes();
        if minutes < MINIMUM_BLOCK_DURATION_MINUTES {
            return Err(TimeBlockValidationError::DurationTooShort { minutes });
        }

        Ok(())
    }

    /// Sorts a slice of [`TimeBlock`] items chronologically in place:
    /// Primarily by `start_time`, secondarily by `end_time`, and tertiarily by `order_index`.
    pub fn sort_chronologically(blocks: &mut [TimeBlock]) {
        blocks.sort_by(|a, b| {
            a.start_time
                .cmp(&b.start_time)
                .then_with(|| a.end_time.cmp(&b.end_time))
                .then_with(|| a.order_index.cmp(&b.order_index))
        });
    }

    /// Validates an entire routine schedule of [`TimeBlock`] items:
    /// 1. Validates each individual block using [`Self::validate_single`].
    /// 2. Sorts blocks chronologically by start time.
    /// 3. Checks adjacent blocks to ensure no two blocks overlap.
    ///
    /// Contiguous boundaries (e.g. 08:00-09:00 and 09:00-10:00) are permitted.
    ///
    /// # Errors
    /// Returns [`TimeBlockValidationError`] if any block is invalid or if an overlap is detected.
    pub fn validate_schedule(blocks: &[TimeBlock]) -> Result<(), TimeBlockValidationError> {
        for block in blocks {
            Self::validate_single(block)?;
        }

        if blocks.len() <= 1 {
            return Ok(());
        }

        let mut sorted: Vec<&TimeBlock> = blocks.iter().collect();
        sorted.sort_by(|a, b| {
            a.start_time
                .cmp(&b.start_time)
                .then_with(|| a.end_time.cmp(&b.end_time))
                .then_with(|| a.order_index.cmp(&b.order_index))
        });

        for window in sorted.windows(2) {
            let prev = window[0];
            let curr = window[1];

            if curr.start_time < prev.end_time {
                return Err(TimeBlockValidationError::OverlapDetected {
                    block_a: prev.name.clone(),
                    start_a: prev.start_time,
                    end_a: prev.end_time,
                    block_b: curr.name.clone(),
                    start_b: curr.start_time,
                    end_b: curr.end_time,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use uuid::Uuid;

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
    fn test_validate_single_valid() {
        let block = mock_block("Work Session", "09:00:00", "10:30:00");
        assert!(TimeBlockValidator::validate_single(&block).is_ok());
    }

    #[test]
    fn test_validate_single_exact_minimum_duration() {
        let block = mock_block("Quick Catchup", "09:00:00", "09:05:00");
        assert!(TimeBlockValidator::validate_single(&block).is_ok());
    }

    #[test]
    fn test_validate_single_duration_too_short() {
        let block = mock_block("Micro Standup", "09:00:00", "09:04:00");
        let result = TimeBlockValidator::validate_single(&block);
        assert_eq!(
            result,
            Err(TimeBlockValidationError::DurationTooShort { minutes: 4 })
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "Block duration (4 mins) is below minimum of 5 minutes"
        );
    }

    #[test]
    fn test_validate_single_invalid_interval_equal() {
        let block = mock_block("Zero Length", "09:00:00", "09:00:00");
        let result = TimeBlockValidator::validate_single(&block);
        let time = NaiveTime::from_hms_opt(9, 0, 0).expect("valid time");
        assert_eq!(
            result,
            Err(TimeBlockValidationError::InvalidInterval {
                start: time,
                end: time,
            })
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "Start time (09:00:00) must be before end time (09:00:00)"
        );
    }

    #[test]
    fn test_validate_single_invalid_interval_inverted() {
        let block = mock_block("Time Travel", "10:00:00", "09:00:00");
        let result = TimeBlockValidator::validate_single(&block);
        let start = NaiveTime::from_hms_opt(10, 0, 0).expect("valid time");
        let end = NaiveTime::from_hms_opt(9, 0, 0).expect("valid time");
        assert_eq!(
            result,
            Err(TimeBlockValidationError::InvalidInterval { start, end })
        );
    }

    #[test]
    fn test_detects_overlapping_blocks() {
        let b1 = mock_block("Morning Prep", "07:00:00", "08:30:00");
        let b2 = mock_block("Deep Work", "08:00:00", "10:00:00");

        let result = TimeBlockValidator::validate_schedule(&[b1, b2]);
        assert!(matches!(
            result,
            Err(TimeBlockValidationError::OverlapDetected { .. })
        ));

        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Overlap detected between 'Morning Prep' (07:00:00-08:30:00) and 'Deep Work' (08:00:00-10:00:00)"));
    }

    #[test]
    fn test_valid_consecutive_schedule() {
        let b1 = mock_block("Morning Prep", "07:00:00", "08:00:00");
        let b2 = mock_block("Deep Work", "08:00:00", "10:00:00");

        assert!(TimeBlockValidator::validate_schedule(&[b1, b2]).is_ok());
    }

    #[test]
    fn test_valid_schedule_with_gaps() {
        let b1 = mock_block("Breakfast", "07:00:00", "07:45:00");
        let b2 = mock_block("Deep Work 1", "08:30:00", "10:30:00");
        let b3 = mock_block("Deep Work 2", "11:00:00", "13:00:00");

        assert!(TimeBlockValidator::validate_schedule(&[b1, b2, b3]).is_ok());
    }

    #[test]
    fn test_empty_and_single_block_schedule() {
        assert!(TimeBlockValidator::validate_schedule(&[]).is_ok());

        let single = mock_block("Solo", "12:00:00", "13:00:00");
        assert!(TimeBlockValidator::validate_schedule(&[single]).is_ok());
    }

    #[test]
    fn test_schedule_validates_individual_blocks() {
        let b1 = mock_block("Valid", "08:00:00", "09:00:00");
        let b2 = mock_block("Too Short", "09:00:00", "09:02:00");

        let result = TimeBlockValidator::validate_schedule(&[b1, b2]);
        assert_eq!(
            result,
            Err(TimeBlockValidationError::DurationTooShort { minutes: 2 })
        );
    }

    #[test]
    fn test_detects_identical_start_times() {
        let b1 = mock_block("Task A", "08:00:00", "09:00:00");
        let b2 = mock_block("Task B", "08:00:00", "08:30:00");

        let result = TimeBlockValidator::validate_schedule(&[b1, b2]);
        assert!(matches!(
            result,
            Err(TimeBlockValidationError::OverlapDetected { .. })
        ));
    }

    #[test]
    fn test_detects_complete_subset_overlap() {
        let b1 = mock_block("Long Session", "08:00:00", "12:00:00");
        let b2 = mock_block("Meeting", "09:00:00", "10:00:00");

        let result = TimeBlockValidator::validate_schedule(&[b1, b2]);
        assert!(matches!(
            result,
            Err(TimeBlockValidationError::OverlapDetected { .. })
        ));
    }

    #[test]
    fn test_detects_overlap_regardless_of_input_order() {
        let b1 = mock_block("First", "07:00:00", "08:30:00");
        let b2 = mock_block("Second", "08:00:00", "09:30:00");

        // Input passed in reversed order
        let result = TimeBlockValidator::validate_schedule(&[b2, b1]);
        assert_eq!(
            result,
            Err(TimeBlockValidationError::OverlapDetected {
                block_a: "First".to_string(),
                start_a: NaiveTime::from_hms_opt(7, 0, 0).expect("valid time"),
                end_a: NaiveTime::from_hms_opt(8, 30, 0).expect("valid time"),
                block_b: "Second".to_string(),
                start_b: NaiveTime::from_hms_opt(8, 0, 0).expect("valid time"),
                end_b: NaiveTime::from_hms_opt(9, 30, 0).expect("valid time"),
            })
        );
    }

    #[test]
    fn test_sort_chronologically() {
        let b1 = mock_block("Lunch", "12:00:00", "13:00:00");
        let b2 = mock_block("Wakeup", "06:30:00", "07:00:00");
        let b3 = mock_block("Morning Work", "08:00:00", "10:00:00");

        let mut blocks = vec![b1, b2, b3];
        TimeBlockValidator::sort_chronologically(&mut blocks);

        assert_eq!(blocks[0].name, "Wakeup");
        assert_eq!(blocks[1].name, "Morning Work");
        assert_eq!(blocks[2].name, "Lunch");
    }
}
