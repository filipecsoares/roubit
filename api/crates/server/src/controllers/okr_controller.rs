//! OKRs & Goal Alignment REST API Controllers (RF03)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    KeyResult, Objective, ObjectiveWithKeyResults, OkrRepository, SqliteOkrRepository,
};

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
    _user: AuthenticatedUser,
    Query(query): Query<ListOkrsQuery>,
) -> Result<Json<Vec<ObjectiveWithKeyResults>>, ApiError> {
    let repo = SqliteOkrRepository::new(state.db);
    let objectives = if let Some(quarter) = query.quarter {
        repo.list_objectives_by_quarter(&quarter)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
    } else {
        repo.list_all_objectives()
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
    _user: AuthenticatedUser,
    Json(payload): Json<CreateObjectiveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let repo = SqliteOkrRepository::new(state.db);
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
    );

    repo.create_objective(&objective)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let mut created_krs = Vec::new();
    if let Some(krs) = payload.key_results {
        for kr_req in krs {
            kr_req
                .validate()
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;

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

            repo.create_key_result(&kr)
                .await
                .map_err(|e| ApiError::Internal(e.into()))?;

            created_krs.push(kr);
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

/// Constructs router for OKR endpoints
pub fn okr_routes() -> Router<AppState> {
    Router::new().route("/", get(list_okrs).post(create_objective))
}
