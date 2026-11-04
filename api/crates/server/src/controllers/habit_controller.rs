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
            "Invalid habit type: '{other}'. Expected binary, numeric, or timer."
        ))),
    }
}

/// Invalidates both L1 in-memory summary cache and L2 Redis cache for `user_id`.
async fn invalidate_habit_caches(state: &AppState, user_id: Uuid) {
    {
        let mut cache = state.summary_cache.write().await;
        cache.remove(&user_id);
    }
    if let Some(redis) = &state.redis {
        if let Err(err) = redis
            .invalidate_user_habits_cache(&user_id.to_string())
            .await
        {
            tracing::warn!(error = %err, %user_id, "Failed to invalidate habit summary cache");
        }
    }
}

/// Retrieves cached habit summary from L1 or L2 cache if available.
async fn get_cached_habit_summary(state: &AppState, user_id: Uuid) -> Option<String> {
    // 1. Check L1 in-memory summary cache first
    {
        let cache_read = state.summary_cache.read().await;
        if let Some((cached_at, ref cached_json)) = cache_read.get(&user_id) {
            if cached_at.elapsed() < std::time::Duration::from_secs(300) {
                return Some(cached_json.clone());
            }
        }
    }

    // 2. Check L2 Redis cache if available
    if let Some(redis) = &state.redis {
        match redis.get_cached_habit_summary(&user_id.to_string()).await {
            Ok(Some(cached_json)) => {
                // Populate L1 cache from Redis hit
                state
                    .summary_cache
                    .write()
                    .await
                    .insert(user_id, (std::time::Instant::now(), cached_json.clone()));
                return Some(cached_json);
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, %user_id, "Redis cache lookup failed; falling back to database");
            }
        }
    }

    None
}

/// Populates L1 and L2 caches with newly queried habit summary JSON.
async fn set_cached_habit_summary(state: &AppState, user_id: Uuid, json_str: &str) {
    state
        .summary_cache
        .write()
        .await
        .insert(user_id, (std::time::Instant::now(), json_str.to_string()));

    if let Some(redis) = &state.redis {
        if let Err(err) = redis
            .set_cached_habit_summary(&user_id.to_string(), json_str, 300)
            .await
        {
            tracing::warn!(error = %err, %user_id, "Failed to cache habit summary in Redis");
        }
    }
}

/// Handler for `GET /api/v1/habits`
///
/// Returns active habits enriched with atomic streak metrics.
/// Utilizes tiered L1 (RAM) and L2 (Redis) caching.
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_habits(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, ApiError> {
    if let Some(cached_json) = get_cached_habit_summary(&state, user.id).await {
        let res = Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .header("x-cache", "HIT")
            .body(Body::from(cached_json))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        return Ok(res);
    }

    let repo = SqliteHabitRepository::new(state.db.clone());
    let today = Utc::now().date_naive();
    let habits_with_streaks = repo
        .list_active_habits_with_streaks(user.id, today)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let result: Vec<HabitWithStreakResponse> = habits_with_streaks
        .into_iter()
        .map(|(habit, streak)| HabitWithStreakResponse { habit, streak })
        .collect();

    let payload_bytes = serde_json::to_vec(&result)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Serialization error: {e}")))?;

    if let Ok(json_str) = std::str::from_utf8(&payload_bytes) {
        set_cached_habit_summary(&state, user.id, json_str).await;
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

    let repo = SqliteHabitRepository::new(state.db.clone());
    repo.create_habit(user.id, &habit)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    invalidate_habit_caches(&state, user.id).await;

    if let Some(ref redis_svc) = state.redis {
        let scheduler = roubit_core::ReminderScheduler::new(std::sync::Arc::new(redis_svc.clone()));
        if let Err(err) = scheduler
            .schedule_reminder(&user.id.to_string(), "habit", &habit.id.to_string(), 3600)
            .await
        {
            tracing::warn!(error = %err, habit_id = %habit.id, "Failed to schedule habit reminder in Redis");
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
    let okr_repo = SqliteOkrRepository::new(state.db.clone());
    let sync_svc = OkrSyncService::new(okr_repo, repo.clone());
    if let Err(err) = sync_svc.sync_linked_krs_for_habit(user.id, habit_id).await {
        tracing::warn!(error = %err, %habit_id, "Failed to synchronize linked Key Results for habit check-in");
    }

    // Invalidate L1 & L2 habit summary cache atomically on check-in
    invalidate_habit_caches(&state, user.id).await;

    // Read updated habit streak atomically computed in O(1)
    let updated_habit = repo
        .get_habit_by_id(habit_id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .unwrap_or(habit);

    let today = Utc::now().date_naive();
    let is_active_today = if log.log_date == today {
        log.is_completed
    } else {
        repo.is_habit_completed_on_date(habit_id, today)
            .await
            .unwrap_or(false)
    };

    let streak = StreakMetrics {
        current_streak: u32::try_from(updated_habit.current_streak.max(0)).unwrap_or(0),
        longest_streak: u32::try_from(updated_habit.longest_streak.max(0)).unwrap_or(0),
        is_active_today,
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
    let repo = SqliteHabitRepository::new(state.db.clone());
    match repo.delete_habit(habit_id, user.id).await {
        Ok(()) => {
            invalidate_habit_caches(&state, user.id).await;
            if let Some(ref redis_svc) = state.redis {
                let scheduler =
                    roubit_core::ReminderScheduler::new(std::sync::Arc::new(redis_svc.clone()));
                let _ = scheduler
                    .cancel_reminder(&user.id.to_string(), "habit", &habit_id.to_string())
                    .await;
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
