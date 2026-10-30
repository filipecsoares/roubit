//! Centralized Error Handling & RFC 7807 Problem Details for HTTP APIs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// RFC 7807 Problem Details response object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProblemDetails {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

/// Centralized API errors mapped to standard HTTP statuses
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", msg.clone()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", msg.clone()),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", msg.clone()),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", msg.clone()),
            Self::Internal(err) => {
                tracing::error!(error = ?err, "Internal unhandled error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        let problem = ProblemDetails {
            r#type: format!("https://roubit.app/errors/{}", status.as_u16()),
            title: title.to_string(),
            status: status.as_u16(),
            detail,
            instance: None,
        };

        (status, Json(problem)).into_response()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_not_found_problem_details() {
        let err = ApiError::NotFound("Routine 123 does not exist".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: ProblemDetails = serde_json::from_slice(&body).expect("Failed to parse JSON");

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert_eq!(problem.detail, "Routine 123 does not exist");
        assert_eq!(problem.r#type, "https://roubit.app/errors/404");
        assert_eq!(problem.instance, None);
    }

    #[tokio::test]
    async fn test_bad_request_problem_details() {
        let err = ApiError::BadRequest("Invalid title".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: ProblemDetails = serde_json::from_slice(&body).expect("Failed to parse JSON");

        assert_eq!(problem.status, 400);
        assert_eq!(problem.title, "Bad Request");
        assert_eq!(problem.detail, "Invalid title");
    }

    #[tokio::test]
    async fn test_unauthorized_problem_details() {
        let err = ApiError::Unauthorized("Invalid token".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: ProblemDetails = serde_json::from_slice(&body).expect("Failed to parse JSON");

        assert_eq!(problem.status, 401);
        assert_eq!(problem.title, "Unauthorized");
    }

    #[tokio::test]
    async fn test_conflict_problem_details() {
        let err = ApiError::Conflict("Email already taken".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: ProblemDetails = serde_json::from_slice(&body).expect("Failed to parse JSON");

        assert_eq!(problem.status, 409);
        assert_eq!(problem.title, "Conflict");
    }

    #[tokio::test]
    async fn test_internal_error_sanitization() {
        let inner_err = anyhow::anyhow!(
            "Sensitive database connection secret leaked: postgres://admin:pass@host"
        );
        let err = ApiError::Internal(inner_err);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let problem: ProblemDetails = serde_json::from_slice(&body).expect("Failed to parse JSON");

        assert_eq!(problem.status, 500);
        assert_eq!(problem.title, "Internal Server Error");
        // Detail must be sanitized and NOT leak sensitive inner error!
        assert_eq!(problem.detail, "An unexpected error occurred");
        assert!(!format!("{problem:?}").contains("postgres://"));
    }
}
