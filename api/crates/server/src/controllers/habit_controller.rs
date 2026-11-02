use std::collections::HashSet;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    Habit, HabitLog, HabitRepository, HabitRepositoryError, HabitType, OkrSyncService,
    SqliteHabitRepository, SqliteOkrRepository, StreakMetrics,
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
/// Returns cached summary from Redis if available, or reads habits and today's completion
/// from SQLite and caches the result for 300 seconds.
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_habits(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, ApiError> {
    let user_id_str = user.id.to_string();

    // 1. Check Redis cache first if Redis service is active
    if let Some(redis) = &state.redis {
        match redis.get_cached_habit_summary(&user_id_str).await {
            Ok(Some(cached_json)) => {
                let res = Response::builder()
                    .status(StatusCode::OK)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .header("x-cache", "HIT")
                    .body(Body::from(cached_json))
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
                return Ok(res);
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, user_id = %user.id, "Redis cache lookup failed; falling back to database");
            }
        }
    }

    // 2. Query SQLite
    let repo = SqliteHabitRepository::new(state.db.clone());
    let habits = repo
        .list_active_habits(user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let today = Utc::now().date_naive();
    let today_completed: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
        r"
        SELECT hl.habit_id
        FROM habit_logs hl
        JOIN habits h ON hl.habit_id = h.id
        WHERE h.user_id = ? AND hl.log_date = ? AND hl.is_completed = 1
        ",
    )
    .bind(user.id)
    .bind(today)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    let mut result = Vec::with_capacity(habits.len());
    for habit in habits {
        let is_active_today = today_completed.contains(&habit.id);
        let streak = StreakMetrics {
            current_streak: u32::try_from(habit.current_streak.max(0)).unwrap_or(0),
            longest_streak: u32::try_from(habit.longest_streak.max(0)).unwrap_or(0),
            is_active_today,
        };
        result.push(HabitWithStreakResponse { habit, streak });
    }

    let payload_bytes = serde_json::to_vec(&result)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Serialization error: {e}")))?;

    // 3. Populate Redis cache if active
    if let Some(redis) = &state.redis {
        if let Ok(json_str) = std::str::from_utf8(&payload_bytes) {
            if let Err(err) = redis
                .set_cached_habit_summary(&user_id_str, json_str, 300)
                .await
            {
                tracing::warn!(error = %err, user_id = %user.id, "Failed to cache habit summary in Redis");
            }
        }
    }

    let res = Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header("x-cache", "MISS")
        .body(Body::from(payload_bytes))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;

    Ok(res)
}

/// Handler for `POST /api/v1/habits`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn create_habit(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateHabitRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let habit_type = parse_habit_type(&payload.habit_type)?;
    let habit = Habit {
        id: Uuid::new_v4(),
        user_id: Some(user.id),
        name: payload.name,
        description: payload.description,
        habit_type,
        target_value: payload.target_value,
        unit: payload.unit,
        frequency_days: payload.frequency_days.unwrap_or(0b0111_1111),
        is_archived: false,
        current_streak: 0,
        longest_streak: 0,
        color_hex: payload.color_hex.unwrap_or_else(|| "#10B981".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let repo = SqliteHabitRepository::new(state.db);
    repo.create_habit(user.id, &habit)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if let Some(redis) = &state.redis {
        if let Err(err) = redis
            .invalidate_user_habits_cache(&user.id.to_string())
            .await
        {
            tracing::warn!(error = %err, user_id = %user.id, "Failed to invalidate habit summary cache after create");
        }
    }

    Ok((StatusCode::CREATED, Json(habit)))
}

/// Handler for `POST /api/v1/habits/:id/checkin`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn record_checkin(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(habit_id): Path<Uuid>,
    Json(payload): Json<HabitCheckinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let repo = SqliteHabitRepository::new(state.db.clone());
    let habit = match repo.get_habit_by_id(habit_id, user.id).await {
        Ok(Some(h)) => h,
        Ok(None) | Err(HabitRepositoryError::NotFound(_)) => {
            return Err(ApiError::NotFound(format!("Habit {habit_id} not found")));
        }
        Err(e) => return Err(ApiError::Internal(e.into())),
    };

    let is_increment = payload.is_increment.unwrap_or(false);
    let log = match repo
        .record_check_in(habit_id, user.id, payload.date, payload.value, is_increment)
        .await
    {
        Ok(l) => l,
        Err(HabitRepositoryError::NotFound(_)) => {
            return Err(ApiError::NotFound(format!("Habit {habit_id} not found")));
        }
        Err(e) => return Err(ApiError::Internal(e.into())),
    };

    // Synchronize linked Key Results progress
    let okr_repo = SqliteOkrRepository::new(state.db);
    let sync_svc = OkrSyncService::new(okr_repo, repo.clone());
    if let Err(err) = sync_svc.sync_linked_krs_for_habit(user.id, habit_id).await {
        tracing::warn!(error = %err, %habit_id, "Failed to synchronize linked Key Results for habit check-in");
    }

    // Invalidate Redis habit summary cache atomically on check-in
    if let Some(redis) = &state.redis {
        if let Err(err) = redis
            .invalidate_user_habits_cache(&user.id.to_string())
            .await
        {
            tracing::warn!(error = %err, user_id = %user.id, "Failed to invalidate habit summary cache after checkin");
        }
    }

    // Read updated habit streak atomically computed in O(1)
    let updated_habit = repo
        .get_habit_by_id(habit_id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .unwrap_or(habit);

    let streak = StreakMetrics {
        current_streak: u32::try_from(updated_habit.current_streak.max(0)).unwrap_or(0),
        longest_streak: u32::try_from(updated_habit.longest_streak.max(0)).unwrap_or(0),
        is_active_today: log.is_completed,
    };

    Ok((StatusCode::OK, Json(CheckinResponse { log, streak })))
}

/// Handler for `DELETE /api/v1/habits/:id`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if habit not found or not owned by user, or [`ApiError::Internal`] on db error.
pub async fn delete_habit(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(habit_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = SqliteHabitRepository::new(state.db);
    match repo.delete_habit(habit_id, user.id).await {
        Ok(()) => {
            if let Some(redis) = &state.redis {
                if let Err(err) = redis
                    .invalidate_user_habits_cache(&user.id.to_string())
                    .await
                {
                    tracing::warn!(error = %err, user_id = %user.id, "Failed to invalidate habit summary cache after delete");
                }
            }
            Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": true }))))
        }
        Err(HabitRepositoryError::NotFound(_)) => {
            Err(ApiError::NotFound(format!("Habit {habit_id} not found")))
        }
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Constructs router for habit endpoints
pub fn habit_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_habits).post(create_habit))
        .route("/{id}/checkin", post(record_checkin))
        .route("/{id}", axum::routing::delete(delete_habit))
}
