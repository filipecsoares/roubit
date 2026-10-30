//! Authentication handlers for user registration, login, and profile access

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        jwt::{encode_jwt, hash_password, verify_password},
    },
    errors::ApiError,
    state::AppState,
};

/// Registration request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Login request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Public user profile DTO
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
}

/// Authentication response containing token and user profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

fn validate_registration(req: &RegisterRequest) -> Result<(String, &str), ApiError> {
    let email = req.email.trim().to_lowercase();
    if email.len() < 3 || !email.contains('@') || !email.contains('.') {
        return Err(ApiError::BadRequest(
            "Invalid email address format".to_string(),
        ));
    }

    if req.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    Ok((email, &req.password))
}

fn build_cookie_header(token: &str) -> Option<HeaderValue> {
    let cookie_val =
        format!("roubit_token={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600");
    HeaderValue::from_str(&cookie_val).ok()
}

/// Handler for `POST /api/v1/auth/register`
///
/// # Errors
/// Returns [`ApiError::BadRequest`] on validation errors, [`ApiError::Conflict`] if email exists,
/// or [`ApiError::Internal`] if database insertion or password hashing fails.
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    let (email, password) = validate_registration(&payload)?;

    let existing = sqlx::query("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    if existing.is_some() {
        return Err(ApiError::Conflict(
            "User with this email already exists".to_string(),
        ));
    }

    let password_hash = hash_password(password)?;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash, created_at)
        VALUES (?, ?, ?, ?)
        ",
    )
    .bind(user_id.to_string())
    .bind(&email)
    .bind(&password_hash)
    .bind(&created_at)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let token = encode_jwt(user_id, &email, &state.config.jwt_secret, 3600)?;
    let user_dto = UserDto { id: user_id, email };

    let body = AuthResponse {
        token: token.clone(),
        user: user_dto,
    };

    let mut response = (StatusCode::CREATED, Json(body)).into_response();
    if let Some(cookie) = build_cookie_header(&token) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }

    Ok(response)
}

/// Handler for `POST /api/v1/auth/login`
///
/// # Errors
/// Returns [`ApiError::Unauthorized`] on invalid email or password, or [`ApiError::Internal`] on db error.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let email = payload.email.trim().to_lowercase();

    let row = sqlx::query("SELECT id, email, password_hash FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(user_row) = row else {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    };

    let id_str: String = user_row.get("id");
    let user_id = Uuid::parse_str(&id_str)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("Invalid user UUID in DB: {e}")))?;
    let password_hash: String = user_row.get("password_hash");

    let is_valid = verify_password(&payload.password, &password_hash)?;
    if !is_valid {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    let token = encode_jwt(user_id, &email, &state.config.jwt_secret, 3600)?;
    let user_dto = UserDto { id: user_id, email };

    let body = AuthResponse {
        token: token.clone(),
        user: user_dto,
    };

    let mut response = (StatusCode::OK, Json(body)).into_response();
    if let Some(cookie) = build_cookie_header(&token) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }

    Ok(response)
}

/// Handler for `GET /api/v1/auth/me`
pub async fn me(user: AuthenticatedUser) -> Json<UserDto> {
    Json(UserDto {
        id: user.id,
        email: user.email,
    })
}
