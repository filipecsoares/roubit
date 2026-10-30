//! Habit Tracking REST API Controllers (RF02)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    Habit, HabitLog, HabitRepository, HabitType, OkrSyncService, SqliteHabitRepository,
    SqliteOkrRepository, StreakCalculator, StreakMetrics,
};

use crate::{auth::AuthenticatedUser, errors::ApiError, state::AppState};

/// Request payload to create a new habit definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateHabitRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub habit_type: String,
    #[validate(range(min = 0.01))]
    pub target_value: f64,
    pub unit: Option<String>,
    pub frequency_days: Option<u8>,
    pub color_hex: Option<String>,
}

/// Request payload for habit check-in
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HabitCheckinRequest {
    pub date: NaiveDate,
    #[validate(range(min = 0.0))]
    pub value: f64,
    pub is_increment: Option<bool>,
}

/// Habit response enriched with streak metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitWithStreakResponse {
    pub habit: Habit,
    pub streak: StreakMetrics,
}

/// Check-in response payload containing recorded log and updated streak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinResponse {
    pub log: HabitLog,
    pub streak: StreakMetrics,
}

fn parse_habit_type(s: &str) -> Result<HabitType, ApiError> {
    match s.trim().to_lowercase().as_str() {
        "binary" => Ok(HabitType::Binary),
        "numeric" => Ok(HabitType::Numeric),
        "timer" => Ok(HabitType::Timer),
        other => Err(ApiError::BadRequest(format!(
            "Invalid habit_type '{other}'. Allowed: binary, numeric, timer"
        ))),
    }
}

/// Handler for `GET /api/v1/habits`
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_habits(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Result<Json<Vec<HabitWithStreakResponse>>, ApiError> {
    let repo = SqliteHabitRepository::new(state.db);
    let habits = repo
        .list_active_habits()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let today = Utc::now().date_naive();
    let mut result = Vec::with_capacity(habits.len());

    for habit in habits {
        let logs = repo
            .get_logs_for_habit(habit.id, today - chrono::Duration::days(365), today)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        let completed_dates: Vec<NaiveDate> = logs
            .iter()
            .filter(|l| l.is_completed)
            .map(|l| l.log_date)
            .collect();

        let streak = StreakCalculator::calculate(&completed_dates, habit.frequency_days, today);
        result.push(HabitWithStreakResponse { habit, streak });
    }

    Ok(Json(result))
}

/// Handler for `POST /api/v1/habits`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn create_habit(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(payload): Json<CreateHabitRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let habit_type = parse_habit_type(&payload.habit_type)?;
    let habit = Habit {
        id: Uuid::new_v4(),
        name: payload.name,
        description: payload.description,
        habit_type,
        target_value: payload.target_value,
        unit: payload.unit,
        frequency_days: payload.frequency_days.unwrap_or(0b0111_1111),
        is_archived: false,
        color_hex: payload.color_hex.unwrap_or_else(|| "#10B981".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let repo = SqliteHabitRepository::new(state.db);
    repo.create_habit(&habit)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((StatusCode::CREATED, Json(habit)))
}

/// Handler for `POST /api/v1/habits/:id/checkin`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn record_checkin(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(habit_id): Path<Uuid>,
    Json(payload): Json<HabitCheckinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let repo = SqliteHabitRepository::new(state.db.clone());
    let habit = repo
        .get_habit_by_id(habit_id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or_else(|| ApiError::NotFound(format!("Habit {habit_id} not found")))?;

    let is_increment = payload.is_increment.unwrap_or(false);
    let log = repo
        .record_check_in(habit_id, payload.date, payload.value, is_increment)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Synchronize linked Key Results progress
    let okr_repo = SqliteOkrRepository::new(state.db);
    let sync_svc = OkrSyncService::new(okr_repo, repo.clone());
    let _ = sync_svc.sync_linked_krs_for_habit(habit_id).await;

    // Recalculate streak
    let logs = repo
        .get_logs_for_habit(
            habit_id,
            payload.date - chrono::Duration::days(365),
            payload.date,
        )
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let completed_dates: Vec<NaiveDate> = logs
        .iter()
        .filter(|l| l.is_completed)
        .map(|l| l.log_date)
        .collect();

    let streak = StreakCalculator::calculate(&completed_dates, habit.frequency_days, payload.date);

    Ok((StatusCode::OK, Json(CheckinResponse { log, streak })))
}

/// Handler for `DELETE /api/v1/habits/:id`
///
/// # Errors
/// Returns [`ApiError::Internal`] on db error.
pub async fn delete_habit(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(habit_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = SqliteHabitRepository::new(state.db);
    repo.delete_habit(habit_id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": true }))))
}

/// Constructs router for habit endpoints
pub fn habit_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_habits).post(create_habit))
        .route("/{id}/checkin", post(record_checkin))
        .route("/{id}", axum::routing::delete(delete_habit))
}
