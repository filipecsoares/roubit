//! OKRs & Goal Alignment REST API Controllers (RF03)

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    DatabasePool, KeyResult, Objective, ObjectiveWithKeyResults, OkrRepositoryError,
    OkrSyncService, PgHabitRepository, PgOkrRepository, SqliteHabitRepository, SqliteOkrRepository,
};

use super::extract_local_date;
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

/// Request payload to update an existing Key Result's habit link or achieved value
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateKeyResultRequest {
    pub linked_habit_id: Option<Uuid>,
    pub current_value: Option<f64>,
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
    headers: HeaderMap,
    Json(payload): Json<CreateObjectiveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let today = extract_local_date(&headers);
    let quarter = payload.quarter.unwrap_or_else(|| {
        let q = (today.month() - 1) / 3 + 1;
        format!("{}-Q{}", today.year(), q)
    });
    let start_date = payload.start_date.unwrap_or(today);
    let end_date = payload
        .end_date
        .unwrap_or(today + chrono::Duration::days(90));

    if start_date > end_date {
        return Err(ApiError::BadRequest(format!(
            "Invalid date range: start_date ({start_date}) cannot be after end_date ({end_date})"
        )));
    }

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

            if let Err(err) = state.okr_repo.create_key_result(user.id, &kr).await {
                let _ = state.okr_repo.delete_objective(objective.id, user.id).await;
                return match err {
                    OkrRepositoryError::Habit(msg) => Err(ApiError::NotFound(msg)),
                    OkrRepositoryError::ObjectiveNotFound(_) => Err(ApiError::NotFound(format!(
                        "Objective {} not found",
                        objective.id
                    ))),
                    e => Err(ApiError::Internal(e.into())),
                };
            }

            if let Some(linked_habit_id) = kr.linked_habit_id {
                let sync_res = match &state.db {
                    DatabasePool::Sqlite(pool) => {
                        let sync_svc = OkrSyncService::new(
                            SqliteOkrRepository::new(pool.clone()),
                            SqliteHabitRepository::new(pool.clone()),
                        );
                        sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
                    }
                    DatabasePool::Postgres(pool) => {
                        let sync_svc = OkrSyncService::new(
                            PgOkrRepository::new(pool.clone()),
                            PgHabitRepository::new(pool.clone()),
                        );
                        sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
                    }
                };
                if let Err(e) = sync_res {
                    tracing::warn!(error = %e, %linked_habit_id, "Sync linked habit progress failed");
                }
            }

            created_krs.push(kr);
        }
    }

    let final_obj = state
        .okr_repo
        .get_objective_with_krs(objective.id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .unwrap_or(ObjectiveWithKeyResults {
            objective,
            key_results: created_krs,
        });

    Ok((StatusCode::CREATED, Json(final_obj)))
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
        Ok(()) => {
            if let Some(linked_habit_id) = kr.linked_habit_id {
                let sync_res = match &state.db {
                    DatabasePool::Sqlite(pool) => {
                        let sync_svc = OkrSyncService::new(
                            SqliteOkrRepository::new(pool.clone()),
                            SqliteHabitRepository::new(pool.clone()),
                        );
                        sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
                    }
                    DatabasePool::Postgres(pool) => {
                        let sync_svc = OkrSyncService::new(
                            PgOkrRepository::new(pool.clone()),
                            PgHabitRepository::new(pool.clone()),
                        );
                        sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
                    }
                };
                if let Err(e) = sync_res {
                    tracing::warn!(error = %e, %linked_habit_id, "Sync linked habit progress failed");
                }
            }
            Ok((StatusCode::CREATED, Json(kr)))
        }
        Err(OkrRepositoryError::ObjectiveNotFound(_)) => Err(ApiError::NotFound(format!(
            "Objective {objective_id} not found"
        ))),
        Err(OkrRepositoryError::Habit(msg)) => Err(ApiError::NotFound(msg)),
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Handler for `PATCH /api/v1/okrs/:id/key-results/:kr_id`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if objective, key result or linked habit not found,
/// [`ApiError::BadRequest`] on validation errors, or [`ApiError::Internal`] on db error.
pub async fn update_key_result(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((objective_id, kr_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateKeyResultRequest>,
) -> Result<Json<ObjectiveWithKeyResults>, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // If linking to a habit, verify the habit exists and belongs to user
    if let Some(habit_id) = payload.linked_habit_id {
        let habit_opt = state
            .habit_repo
            .get_habit_by_id(habit_id, user.id)
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
        if habit_opt.is_none() {
            return Err(ApiError::NotFound(format!("Habit {habit_id} not found")));
        }
    }

    let _updated_kr = state
        .okr_repo
        .update_key_result_link_or_value(
            user.id,
            kr_id,
            payload.linked_habit_id.map(Some),
            payload.current_value,
        )
        .await
        .map_err(|e| match e {
            OkrRepositoryError::KeyResultNotFound(id) => {
                ApiError::NotFound(format!("Key Result {id} not found"))
            }
            OkrRepositoryError::ObjectiveNotFound(id) => {
                ApiError::NotFound(format!("Objective {id} not found"))
            }
            other => ApiError::Internal(other.into()),
        })?;

    // If habit link was specified in the request, sync progress immediately
    if let Some(linked_habit_id) = payload.linked_habit_id {
        let sync_res = match &state.db {
            DatabasePool::Sqlite(pool) => {
                let sync_svc = OkrSyncService::new(
                    SqliteOkrRepository::new(pool.clone()),
                    SqliteHabitRepository::new(pool.clone()),
                );
                sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
            }
            DatabasePool::Postgres(pool) => {
                let sync_svc = OkrSyncService::new(
                    PgOkrRepository::new(pool.clone()),
                    PgHabitRepository::new(pool.clone()),
                );
                sync_svc.sync_linked_krs_for_habit(user.id, linked_habit_id).await
            }
        };
        if let Err(e) = sync_res {
            tracing::warn!(error = %e, %linked_habit_id, "Sync linked habit progress failed");
        }
    }

    let final_obj = state
        .okr_repo
        .get_objective_with_krs(objective_id, user.id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or_else(|| ApiError::NotFound(format!("Objective {objective_id} not found")))?;

    Ok(Json(final_obj))
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
        .route("/{id}/key-results/{kr_id}", patch(update_key_result))
}
