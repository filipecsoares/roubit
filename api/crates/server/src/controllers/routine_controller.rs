//! Routine & Time-Blocking REST API Controllers (RF01)

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    BlockStatus, CurrentRoutineProgress, Routine, RoutineProgressTracker, RoutineRepositoryError,
    TimeBlock, TimeBlockValidator,
};

use super::{extract_local_date, extract_local_time};
use crate::{auth::AuthenticatedUser, errors::ApiError, state::AppState};

/// Request payload to create an individual time block within a routine
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTimeBlockRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub category: Option<String>,
    pub color_hex: Option<String>,
    pub order_index: Option<i32>,
}

/// Request payload to create a new routine template with time blocks
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRoutineRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub blocks: Vec<CreateTimeBlockRequest>,
}

/// Request payload to log or update status of a time block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogBlockStatusRequest {
    pub date: Option<NaiveDate>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

/// Response payload containing routine definition and current real-time progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineWithProgressResponse {
    pub routine: Option<Routine>,
    pub blocks: Vec<TimeBlock>,
    pub progress: Option<CurrentRoutineProgress>,
    #[serde(default)]
    pub completed_block_ids: Vec<Uuid>,
    #[serde(default)]
    pub skipped_block_ids: Vec<Uuid>,
}

fn parse_time(time_str: &str) -> Result<NaiveTime, ApiError> {
    NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
        .map_err(|_| {
            ApiError::BadRequest(format!(
                "Invalid time format '{time_str}'. Expected HH:MM:SS or HH:MM"
            ))
        })
}

/// Handler for `GET /api/v1/routines`
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn get_routine(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<RoutineWithProgressResponse>, ApiError> {
    let routine_opt = state
        .routine_repo
        .get_active_routine(user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(routine) = routine_opt else {
        return Ok(Json(RoutineWithProgressResponse {
            routine: None,
            blocks: Vec::new(),
            progress: None,
            completed_block_ids: Vec::new(),
            skipped_block_ids: Vec::new(),
        }));
    };

    let blocks = state
        .routine_repo
        .list_blocks_for_routine(routine.id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let today = extract_local_date(&headers);
    let daily_logs = state
        .routine_repo
        .list_daily_logs_for_routine(routine.id, user.id, today)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let mut completed_block_ids = Vec::new();
    let mut skipped_block_ids = Vec::new();
    let mut log_status_map = HashMap::with_capacity(daily_logs.len());
    for log in daily_logs {
        match log.status {
            BlockStatus::Completed => completed_block_ids.push(log.time_block_id),
            BlockStatus::Skipped => skipped_block_ids.push(log.time_block_id),
            _ => {}
        }
        log_status_map.insert(log.time_block_id, log.status);
    }

    let block_statuses: Vec<(TimeBlock, BlockStatus)> = blocks
        .iter()
        .map(|block| {
            let status = log_status_map
                .get(&block.id)
                .copied()
                .unwrap_or(BlockStatus::Pending);
            (block.clone(), status)
        })
        .collect();

    let now_time = extract_local_time(&headers);
    let progress = RoutineProgressTracker::compute_progress(&blocks, &block_statuses, now_time);

    Ok(Json(RoutineWithProgressResponse {
        routine: Some(routine),
        blocks,
        progress: Some(progress),
        completed_block_ids,
        skipped_block_ids,
    }))
}

/// Handler for `POST /api/v1/routines`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn create_routine(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateRoutineRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let routine_id = Uuid::new_v4();
    let mut domain_blocks = Vec::with_capacity(payload.blocks.len());

    for (idx, b) in payload.blocks.into_iter().enumerate() {
        b.validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        let start = parse_time(&b.start_time)?;
        let end = parse_time(&b.end_time)?;

        let block = TimeBlock {
            id: Uuid::new_v4(),
            routine_id,
            name: b.name,
            start_time: start,
            end_time: end,
            category: b.category.unwrap_or_else(|| "General".to_string()),
            color_hex: b.color_hex.unwrap_or_else(|| "#6366F1".to_string()),
            order_index: b
                .order_index
                .unwrap_or_else(|| i32::try_from(idx).unwrap_or(0)),
            created_at: Utc::now(),
        };
        domain_blocks.push(block);
    }

    TimeBlockValidator::validate_schedule(&domain_blocks)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let routine = Routine {
        id: routine_id,
        user_id: Some(user.id),
        name: payload.name,
        description: payload.description,
        is_active: payload.is_active.unwrap_or(true),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    state
        .routine_repo
        .create_routine_with_blocks(user.id, &routine, &domain_blocks)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if routine.is_active {
        state
            .routine_repo
            .set_active_routine(routine.id, user.id)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;

        if let Some(ref redis_svc) = state.redis {
            let scheduler =
                roubit_core::ReminderScheduler::new(std::sync::Arc::new(redis_svc.clone()));
            let now_time = Utc::now().time();
            let user_id_str = user.id.to_string();
            for block in &domain_blocks {
                let diff_secs = block
                    .start_time
                    .signed_duration_since(now_time)
                    .num_seconds();
                let fire_in_secs = if diff_secs > 0 {
                    usize::try_from(diff_secs).unwrap_or(60)
                } else {
                    usize::try_from(86_400 + diff_secs).unwrap_or(3600)
                };
                if let Err(err) = scheduler
                    .schedule_reminder(
                        &user_id_str,
                        "time_block",
                        &block.id.to_string(),
                        fire_in_secs.max(1),
                    )
                    .await
                {
                    tracing::warn!(error = %err, block_id = %block.id, "Failed to schedule time_block reminder in Redis");
                }
            }
        }
    }

    Ok((StatusCode::CREATED, Json(routine)))
}

/// Handler for `GET /api/v1/routines/all`
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_routines(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Routine>>, ApiError> {
    let routines = state
        .routine_repo
        .list_routines(user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(routines))
}

/// Handler for `POST /api/v1/routines/blocks/:id/complete`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if block not found or not owned by user, [`ApiError::BadRequest`] on invalid status, or [`ApiError::Internal`] on db error.
pub async fn complete_block(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(block_id): Path<Uuid>,
    headers: HeaderMap,
    payload: Option<Json<LogBlockStatusRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let req = payload.map_or(
        LogBlockStatusRequest {
            date: None,
            status: None,
            notes: None,
        },
        |Json(p)| p,
    );

    let target_date = req.date.unwrap_or_else(|| extract_local_date(&headers));
    let status = match req.status.as_deref().unwrap_or("completed") {
        "completed" => BlockStatus::Completed,
        "in_progress" => BlockStatus::InProgress,
        "skipped" => BlockStatus::Skipped,
        "pending" => BlockStatus::Pending,
        other => {
            return Err(ApiError::BadRequest(format!(
                "Invalid status '{other}'. Allowed: completed, in_progress, skipped, pending"
            )))
        }
    };

    match state
        .routine_repo
        .log_block_status(block_id, user.id, target_date, status, req.notes)
        .await
    {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "time_block_id": block_id,
                "date": target_date,
                "status": status.as_str()
            })),
        )),
        Err(RoutineRepositoryError::NotFound(_) | RoutineRepositoryError::BlockNotFound(_)) => Err(
            ApiError::NotFound(format!("Time block {block_id} not found")),
        ),
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Handler for `DELETE /api/v1/routines/:id`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if routine not found or not owned by user, or [`ApiError::Internal`] on db error.
pub async fn delete_routine(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let blocks_to_cancel = if state.redis.is_some() {
        state
            .routine_repo
            .list_blocks_for_routine(id, user.id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    match state.routine_repo.delete_routine(id, user.id).await {
        Ok(()) => {
            if let Some(ref redis_svc) = state.redis {
                let scheduler =
                    roubit_core::ReminderScheduler::new(std::sync::Arc::new(redis_svc.clone()));
                let user_id_str = user.id.to_string();
                for block in blocks_to_cancel {
                    let _ = scheduler
                        .cancel_reminder(&user_id_str, "time_block", &block.id.to_string())
                        .await;
                }
            }
            Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": true }))))
        }
        Err(RoutineRepositoryError::NotFound(_)) => {
            Err(ApiError::NotFound(format!("Routine {id} not found")))
        }
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Handler for `GET /api/v1/routines/:id/blocks`
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_routine_blocks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(routine_id): Path<Uuid>,
) -> Result<Json<Vec<TimeBlock>>, ApiError> {
    let blocks = state
        .routine_repo
        .list_blocks_for_routine(routine_id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(blocks))
}

/// Constructs router for routine endpoints
pub fn routine_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_routine).post(create_routine))
        .route("/all", get(list_routines))
        .route("/{id}", axum::routing::delete(delete_routine))
        .route("/{id}/blocks", get(list_routine_blocks))
        .route("/blocks/{id}/complete", post(complete_block))
}
