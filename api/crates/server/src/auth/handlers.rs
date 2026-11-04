//! Authentication handlers for user registration, login, and profile access

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use roubit_core::{User, UserRepositoryError};

use crate::{
    auth::{
        extractor::AuthenticatedUser,
        jwt::{encode_jwt, hash_password_async, verify_password_async, DUMMY_ARGON2_HASH},
    },
    errors::ApiError,
    state::AppState,
};

/// Maximum password length in bytes to mitigate Argon2id `DoS` attacks
pub const MAX_PASSWORD_BYTES: usize = 128;

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

    if req.password.len() > MAX_PASSWORD_BYTES {
        return Err(ApiError::BadRequest(format!(
            "Password must not exceed {MAX_PASSWORD_BYTES} characters"
        )));
    }

    Ok((email, &req.password))
}

fn is_cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE").map_or(true, |v| v != "false" && v != "0")
}

fn build_cookie_header(token: &str) -> Option<HeaderValue> {
    let secure_flag = if is_cookie_secure() { "; Secure" } else { "" };
    let cookie_val = format!(
        "roubit_token={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600{secure_flag}"
    );
    HeaderValue::from_str(&cookie_val).ok()
}

fn build_logout_cookie_header() -> Option<HeaderValue> {
    let secure_flag = if is_cookie_secure() { "; Secure" } else { "" };
    let cookie_val =
        format!("roubit_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0{secure_flag}");
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

    if state
        .user_repo
        .find_by_email(&email)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .is_some()
    {
        return Err(ApiError::Conflict(
            "User with this email already exists".to_string(),
        ));
    }

    let password_hash = hash_password_async(password.to_string()).await?;
    let user_id = Uuid::new_v4();
    let created_at = chrono::Utc::now().to_rfc3339();
    let new_user = User::new(user_id, email.clone(), password_hash, created_at);

    state
        .user_repo
        .create_user(&new_user)
        .await
        .map_err(|e| match e {
            UserRepositoryError::AlreadyExists(_) => {
                ApiError::Conflict("User with this email already exists".to_string())
            }
            other => ApiError::Internal(other.into()),
        })?;

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
/// Returns [`ApiError::BadRequest`] if password exceeds maximum length,
/// [`ApiError::Unauthorized`] on invalid email or password, or [`ApiError::Internal`] on db error.
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    if payload.password.len() > MAX_PASSWORD_BYTES {
        return Err(ApiError::BadRequest(format!(
            "Password must not exceed {MAX_PASSWORD_BYTES} characters"
        )));
    }

    let email = payload.email.trim().to_lowercase();

    let Some(user) = state
        .user_repo
        .find_by_email(&email)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
    else {
        // Constant-time defense: run dummy Argon2id verification to avoid email enumeration timing attacks
        let _ = verify_password_async(payload.password, DUMMY_ARGON2_HASH.clone()).await;
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    };

    let is_valid = verify_password_async(payload.password, user.password_hash).await?;
    if !is_valid {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    let token = encode_jwt(user.id, &user.email, &state.config.jwt_secret, 3600)?;
    let user_dto = UserDto {
        id: user.id,
        email: user.email,
    };

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

/// Handler for `POST /api/v1/auth/logout`
///
/// Clears the `roubit_token` session cookie by setting `Max-Age=0`.
pub async fn logout() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    if let Some(cookie) = build_logout_cookie_header() {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}
