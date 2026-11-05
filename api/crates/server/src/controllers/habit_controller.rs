use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    DatabasePool, Habit, HabitLog, HabitRepositoryError, HabitType, OkrSyncService,
    PgHabitRepository, PgOkrRepository, SqliteHabitRepository, SqliteOkrRepository, StreakMetrics,
};

use super::extract_local_date;
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
    #[serde(default)]
    pub today_value: f64,
}

/// Aggregated analytics response for 365-day heatmap and per-habit metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitAnalyticsResponse {
    pub aggregated_activity_map: std::collections::BTreeMap<NaiveDate, u32>,
    pub habits_summary: Vec<HabitSummaryAnalyticsDto>,
}

/// Per-habit consistency and adherence summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitSummaryAnalyticsDto {
    pub habit_id: Uuid,
    pub habit_name: String,
    pub summary: roubit_core::HabitAnalyticsSummary,
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

const MAX_L1_CACHE_ENTRIES: usize = 1000;
const L1_CACHE_TTL_SECS: u64 = 300;

/// Invalidates both L1 in-memory summary cache and L2 Redis cache for `user_id`.
async fn invalidate_habit_caches(state: &AppState, user_id: Uuid) {
    {
        let mut cache = state.summary_cache.write().await;
        cache.retain(|(u, _), _| *u != user_id);
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

async fn set_l1_cache(state: &AppState, user_id: Uuid, today: NaiveDate, json_str: String) {
    let mut cache = state.summary_cache.write().await;
    if cache.len() >= MAX_L1_CACHE_ENTRIES {
        let now = std::time::Instant::now();
        let ttl = std::time::Duration::from_secs(L1_CACHE_TTL_SECS);
        cache.retain(|_, (cached_at, _)| now.duration_since(*cached_at) < ttl);

        if cache.len() >= MAX_L1_CACHE_ENTRIES {
            if let Some((&oldest_key, _)) = cache.iter().min_by_key(|(_, (ts, _))| *ts) {
                cache.remove(&oldest_key);
            }
        }
    }
    cache.insert((user_id, today), (std::time::Instant::now(), json_str));
}

/// Retrieves cached habit summary from L1 or L2 cache if available for a given date.
async fn get_cached_habit_summary(state: &AppState, user_id: Uuid, today: NaiveDate) -> Option<String> {
    // 1. Check L1 in-memory summary cache first
    {
        let cache_read = state.summary_cache.read().await;
        if let Some((cached_at, ref cached_json)) = cache_read.get(&(user_id, today)) {
            if cached_at.elapsed() < std::time::Duration::from_secs(L1_CACHE_TTL_SECS) {
                return Some(cached_json.clone());
            }
        }
    }

    // 2. Check L2 Redis cache if available
    if let Some(redis) = &state.redis {
        match redis.get_cached_habit_summary(&user_id.to_string(), today).await {
            Ok(Some(cached_json)) => {
                // Populate L1 cache from Redis hit
                set_l1_cache(state, user_id, today, cached_json.clone()).await;
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
async fn set_cached_habit_summary(state: &AppState, user_id: Uuid, today: NaiveDate, json_str: &str) {
    set_l1_cache(state, user_id, today, json_str.to_string()).await;

    if let Some(redis) = &state.redis {
        if let Err(err) = redis
            .set_cached_habit_summary(&user_id.to_string(), today, json_str, 300)
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
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let today = extract_local_date(&headers);

    if let Some(cached_json) = get_cached_habit_summary(&state, user.id, today).await {
        let res = Response::builder()
            .status(StatusCode::OK)
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .header("x-cache", "HIT")
            .body(Body::from(cached_json))
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("{e}")))?;
        return Ok(res);
    }
    let habits_with_streaks = state
        .habit_repo
        .list_active_habits_with_streaks(user.id, today)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let mut result: Vec<HabitWithStreakResponse> = Vec::with_capacity(habits_with_streaks.len());
    for (habit, streak) in habits_with_streaks {
        let today_logs = state
            .habit_repo
            .get_logs_for_habit(habit.id, user.id, today, today)
            .await
            .unwrap_or_default();
        let today_value = today_logs.first().map_or(0.0, |l| l.actual_value);
        result.push(HabitWithStreakResponse {
            habit,
            streak,
            today_value,
        });
    }

    let payload_bytes = serde_json::to_vec(&result)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Serialization error: {e}")))?;

    if let Ok(json_str) = std::str::from_utf8(&payload_bytes) {
        set_cached_habit_summary(&state, user.id, today, json_str).await;
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

    state
        .habit_repo
        .create_habit(user.id, &habit)
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
    headers: HeaderMap,
    Json(payload): Json<HabitCheckinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let is_increment = payload.is_increment.unwrap_or(false);
    let log = match state
        .habit_repo
        .record_check_in(habit_id, user.id, payload.date, payload.value, is_increment)
        .await
    {
        Ok(l) => l,
        Err(HabitRepositoryError::NotFound(_)) => {
            return Err(ApiError::NotFound(format!("Habit {habit_id} not found")));
        }
        Err(e) => return Err(ApiError::Internal(e.into())),
    };

    // Synchronize linked Key Results progress across SQLite or PostgreSQL
    let sync_res = match &state.db {
        DatabasePool::Sqlite(pool) => {
            let sync_svc = OkrSyncService::new(
                SqliteOkrRepository::new(pool.clone()),
                SqliteHabitRepository::new(pool.clone()),
            );
            sync_svc.sync_linked_krs_for_habit(user.id, habit_id).await
        }
        DatabasePool::Postgres(pool) => {
            let sync_svc = OkrSyncService::new(
                PgOkrRepository::new(pool.clone()),
                PgHabitRepository::new(pool.clone()),
            );
            sync_svc.sync_linked_krs_for_habit(user.id, habit_id).await
        }
    };
    if let Err(err) = sync_res {
        tracing::warn!(error = %err, %habit_id, "Failed to synchronize linked Key Results for habit check-in");
    }

    // Invalidate L1 & L2 habit summary cache atomically on check-in
    invalidate_habit_caches(&state, user.id).await;

    // Read updated habit streak atomically computed in O(1)
    let updated_habit = state
        .habit_repo
        .get_habit_by_id(habit_id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or_else(|| ApiError::NotFound(format!("Habit {habit_id} not found")))?;

    let today = extract_local_date(&headers);
    let is_active_today = if log.log_date == today {
        log.is_completed
    } else {
        state
            .habit_repo
            .is_habit_completed_on_date(habit_id, today)
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

/// Handler for `GET /api/v1/habits/analytics`
///
/// Returns 365-day aggregated activity heatmap and individual habit adherence summaries.
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn get_habit_analytics(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<HabitAnalyticsResponse>, ApiError> {
    let today = extract_local_date(&headers);
    let start_365d = today - chrono::Duration::days(364);

    let habits = state
        .habit_repo
        .list_all_habits(user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let mut aggregated_activity_map = std::collections::BTreeMap::new();
    let mut habits_summary = Vec::with_capacity(habits.len());

    for habit in habits {
        let logs = state
            .habit_repo
            .get_logs_for_habit(habit.id, user.id, start_365d, today)
            .await
            .unwrap_or_default();

        for log in &logs {
            if log.is_completed {
                *aggregated_activity_map.entry(log.log_date).or_insert(0) += 1;
            }
        }

        let summary = roubit_core::HabitAnalyticsService::compute_summary(&habit, &logs, today);
        habits_summary.push(HabitSummaryAnalyticsDto {
            habit_id: habit.id,
            habit_name: habit.name,
            summary,
        });
    }

    Ok(Json(HabitAnalyticsResponse {
        aggregated_activity_map,
        habits_summary,
    }))
}

/// Handler for `POST /api/v1/habits/:id/archive`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if habit not found or not owned by user, or [`ApiError::Internal`] on db error.
pub async fn archive_habit(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(habit_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    match state.habit_repo.archive_habit(habit_id, user.id).await {
        Ok(()) => {
            invalidate_habit_caches(&state, user.id).await;
            if let Some(ref redis_svc) = state.redis {
                let scheduler =
                    roubit_core::ReminderScheduler::new(std::sync::Arc::new(redis_svc.clone()));
                let _ = scheduler
                    .cancel_reminder(&user.id.to_string(), "habit", &habit_id.to_string())
                    .await;
            }
            Ok((StatusCode::OK, Json(serde_json::json!({ "archived": true }))))
        }
        Err(HabitRepositoryError::NotFound(_)) => {
            Err(ApiError::NotFound(format!("Habit {habit_id} not found")))
        }
        Err(e) => Err(ApiError::Internal(e.into())),
    }
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
    match state.habit_repo.delete_habit(habit_id, user.id).await {
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
        .route("/analytics", get(get_habit_analytics))
        .route("/{id}/checkin", post(record_checkin))
        .route("/{id}/archive", post(archive_habit))
        .route("/{id}", axum::routing::delete(delete_habit))
}
