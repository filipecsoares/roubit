//! Routine & Time-Blocking REST API Controllers (RF01)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    BlockStatus, CurrentRoutineProgress, Routine, RoutineProgressTracker, RoutineRepository,
    SqliteRoutineRepository, TimeBlock, TimeBlockValidator,
};

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
    _user: AuthenticatedUser,
) -> Result<Json<RoutineWithProgressResponse>, ApiError> {
    let repo = SqliteRoutineRepository::new(state.db);
    let routine_opt = repo
        .get_active_routine()
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(routine) = routine_opt else {
        return Ok(Json(RoutineWithProgressResponse {
            routine: None,
            blocks: Vec::new(),
            progress: None,
        }));
    };

    let blocks = repo
        .list_blocks_for_routine(routine.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let today = Utc::now().date_naive();
    let mut block_statuses = Vec::with_capacity(blocks.len());
    for block in &blocks {
        let status = match repo.get_daily_log(block.id, today).await {
            Ok(Some(log)) => log.status,
            _ => BlockStatus::Pending,
        };
        block_statuses.push((block.clone(), status));
    }

    let progress =
        RoutineProgressTracker::compute_progress(&blocks, &block_statuses, Utc::now().time());

    Ok(Json(RoutineWithProgressResponse {
        routine: Some(routine),
        blocks,
        progress: Some(progress),
    }))
}

/// Handler for `POST /api/v1/routines`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn create_routine(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
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
        name: payload.name,
        description: payload.description,
        is_active: payload.is_active.unwrap_or(true),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let repo = SqliteRoutineRepository::new(state.db);
    repo.create_routine_with_blocks(&routine, &domain_blocks)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((StatusCode::CREATED, Json(routine)))
}

/// Handler for `POST /api/v1/routines/blocks/:id/complete`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on invalid status or [`ApiError::Internal`] on db error.
pub async fn complete_block(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(block_id): Path<Uuid>,
    Json(payload): Json<Option<LogBlockStatusRequest>>,
) -> Result<impl IntoResponse, ApiError> {
    let req = payload.unwrap_or(LogBlockStatusRequest {
        date: None,
        status: None,
        notes: None,
    });

    let target_date = req.date.unwrap_or_else(|| Utc::now().date_naive());
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

    let repo = SqliteRoutineRepository::new(state.db);
    repo.log_block_status(block_id, target_date, status, req.notes)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "time_block_id": block_id,
            "date": target_date,
            "status": status.as_str()
        })),
    ))
}

/// Constructs router for routine endpoints
pub fn routine_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_routine).post(create_routine))
        .route("/blocks/{id}/complete", post(complete_block))
}
