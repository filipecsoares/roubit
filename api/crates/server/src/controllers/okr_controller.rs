//! OKRs & Goal Alignment REST API Controllers (RF03)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{KeyResult, Objective, ObjectiveWithKeyResults, OkrRepositoryError};

use crate::{auth::AuthenticatedUser, errors::ApiError, state::AppState};

/// Query parameters for listing objectives
#[derive(Debug, Clone, Deserialize)]
pub struct ListOkrsQuery {
    pub quarter: Option<String>,
}

/// Request payload to create an individual Key Result under an Objective
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateKeyResultRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub initial_value: Option<f64>,
    #[validate(range(min = 0.01))]
    pub target_value: f64,
    pub unit: Option<String>,
    pub weight: Option<f32>,
    pub linked_habit_id: Option<Uuid>,
}

/// Request payload to create an Objective with optional nested Key Results
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateObjectiveRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub quarter: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub key_results: Option<Vec<CreateKeyResultRequest>>,
}

/// Handler for `GET /api/v1/okrs`
///
/// # Errors
/// Returns [`ApiError::Internal`] if database queries fail.
pub async fn list_okrs(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListOkrsQuery>,
) -> Result<Json<Vec<ObjectiveWithKeyResults>>, ApiError> {
    let objectives = if let Some(quarter) = query.quarter {
        state
            .okr_repo
            .list_objectives_by_quarter(user.id, &quarter)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
    } else {
        state
            .okr_repo
            .list_all_objectives(user.id)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
    };

    Ok(Json(objectives))
}

/// Handler for `POST /api/v1/okrs`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors or [`ApiError::Internal`] on db error.
pub async fn create_objective(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<CreateObjectiveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let today = Utc::now().date_naive();
    let quarter = payload.quarter.unwrap_or_else(|| "2026-Q4".to_string());
    let start_date = payload.start_date.unwrap_or(today);
    let end_date = payload
        .end_date
        .unwrap_or(today + chrono::Duration::days(90));

    let objective = Objective::new(
        payload.title,
        payload.description,
        quarter,
        start_date,
        end_date,
    )
    .with_user_id(user.id);

    // Validate all nested key results upfront to prevent orphaned objectives on validation error
    if let Some(ref krs) = payload.key_results {
        for kr_req in krs {
            kr_req
                .validate()
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }

    state
        .okr_repo
        .create_objective(user.id, &objective)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let mut created_krs = Vec::new();
    if let Some(krs) = payload.key_results {
        for kr_req in krs {
            let kr = if let Some(linked_habit_id) = kr_req.linked_habit_id {
                KeyResult::new_habit_linked(
                    objective.id,
                    linked_habit_id,
                    kr_req.title,
                    kr_req.target_value,
                    kr_req.unit.unwrap_or_else(|| "%".to_string()),
                    kr_req.weight.unwrap_or(1.0),
                )
            } else {
                KeyResult::new_numeric(
                    objective.id,
                    kr_req.title,
                    kr_req.initial_value.unwrap_or(0.0),
                    kr_req.target_value,
                    kr_req.unit.unwrap_or_else(|| "%".to_string()),
                    kr_req.weight.unwrap_or(1.0),
                )
            };

            match state.okr_repo.create_key_result(user.id, &kr).await {
                Ok(()) => created_krs.push(kr),
                Err(OkrRepositoryError::Habit(msg)) => return Err(ApiError::NotFound(msg)),
                Err(OkrRepositoryError::ObjectiveNotFound(_)) => {
                    return Err(ApiError::NotFound(format!(
                        "Objective {} not found",
                        objective.id
                    )));
                }
                Err(e) => return Err(ApiError::Internal(e.into())),
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(ObjectiveWithKeyResults {
            objective,
            key_results: created_krs,
        }),
    ))
}

/// Handler for `POST /api/v1/okrs/:id/key-results`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if objective or linked habit not found, [`ApiError::BadRequest`] on validation error, or [`ApiError::Internal`] on db error.
pub async fn create_kr(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(objective_id): Path<Uuid>,
    Json(payload): Json<CreateKeyResultRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let kr = if let Some(linked_habit_id) = payload.linked_habit_id {
        KeyResult::new_habit_linked(
            objective_id,
            linked_habit_id,
            payload.title,
            payload.target_value,
            payload.unit.unwrap_or_else(|| "%".to_string()),
            payload.weight.unwrap_or(1.0),
        )
    } else {
        KeyResult::new_numeric(
            objective_id,
            payload.title,
            payload.initial_value.unwrap_or(0.0),
            payload.target_value,
            payload.unit.unwrap_or_else(|| "%".to_string()),
            payload.weight.unwrap_or(1.0),
        )
    };

    match state.okr_repo.create_key_result(user.id, &kr).await {
        Ok(()) => Ok((StatusCode::CREATED, Json(kr))),
        Err(OkrRepositoryError::ObjectiveNotFound(_)) => Err(ApiError::NotFound(format!(
            "Objective {objective_id} not found"
        ))),
        Err(OkrRepositoryError::Habit(msg)) => Err(ApiError::NotFound(msg)),
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Handler for `DELETE /api/v1/okrs/:id`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if objective not found or not owned by user, or [`ApiError::Internal`] on db error.
pub async fn delete_objective(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    match state.okr_repo.delete_objective(id, user.id).await {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": true })))),
        Err(OkrRepositoryError::ObjectiveNotFound(_)) => {
            Err(ApiError::NotFound(format!("Objective {id} not found")))
        }
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Constructs router for OKR endpoints
pub fn okr_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_okrs).post(create_objective))
        .route("/{id}", axum::routing::delete(delete_objective))
        .route("/{id}/key-results", axum::routing::post(create_kr))
}
