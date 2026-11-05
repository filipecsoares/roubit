//! Domain REST API Controllers

pub mod habit_controller;
pub mod journal_controller;
pub mod okr_controller;
pub mod routine_controller;

pub use habit_controller::{
    habit_routes, CheckinResponse, CreateHabitRequest, HabitAnalyticsResponse, HabitCheckinRequest,
    HabitSummaryAnalyticsDto, HabitWithStreakResponse,
};
pub use journal_controller::{
    journal_routes, EncryptedJournalResponse, JournalMetadataQuery, SaveEncryptedJournalRequest,
};
pub use okr_controller::{
    okr_routes, CreateKeyResultRequest, CreateObjectiveRequest, ListOkrsQuery,
    UpdateKeyResultRequest,
};
pub use routine_controller::{
    complete_block, create_routine, get_routine, list_routine_blocks, routine_routes,
    CreateRoutineRequest, CreateTimeBlockRequest, LogBlockStatusRequest,
    RoutineWithProgressResponse,
};

use axum::http::HeaderMap;
use chrono::{NaiveDate, NaiveTime, Utc};

/// Extracts the client local date from `X-Local-Date` or `X-Timezone` headers,
/// falling back to `Utc::now().date_naive()` if absent or invalid.
#[must_use]
pub fn extract_local_date(headers: &HeaderMap) -> NaiveDate {
    if let Some(date_header) = headers.get("x-local-date") {
        if let Ok(date_str) = date_header.to_str() {
            if let Ok(d) = NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d") {
                return d;
            }
        }
    }

    if let Some(tz_header) = headers.get("x-timezone") {
        if let Ok(tz_str) = tz_header.to_str() {
            if let Ok(tz) = tz_str.trim().parse::<chrono_tz::Tz>() {
                return Utc::now().with_timezone(&tz).date_naive();
            }
        }
    }

    Utc::now().date_naive()
}

/// Extracts the client local time from the `X-Timezone` header,
/// falling back to `Utc::now().time()` if absent or invalid.
#[must_use]
pub fn extract_local_time(headers: &HeaderMap) -> NaiveTime {
    if let Some(tz_header) = headers.get("x-timezone") {
        if let Ok(tz_str) = tz_header.to_str() {
            if let Ok(tz) = tz_str.trim().parse::<chrono_tz::Tz>() {
                return Utc::now().with_timezone(&tz).time();
            }
        }
    }

    Utc::now().time()
}
