//! Zero-Knowledge Encrypted Journal REST API Controllers (RF04)
//!
//! Provides API endpoints for encrypted reflection storage. The server strictly enforces
//! a zero-knowledge architecture: it accepts, stores, and serves only pre-encrypted envelopes
//! (salt, nonce, ciphertext) and unencrypted query metadata (date, mood, tags).
//! Plaintext journal reflections never touch the server unencrypted.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::prelude::*;
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use roubit_core::{
    EncryptedJournalEntry, JournalMetadata, JournalRepository, JournalRepositoryError,
    SqliteJournalRepository,
};

use crate::{auth::AuthenticatedUser, errors::ApiError, state::AppState};

/// Query parameters for listing journal metadata across a date range
#[derive(Debug, Clone, Deserialize)]
pub struct JournalMetadataQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// Request payload to persist a pre-encrypted journal entry envelope
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SaveEncryptedJournalRequest {
    pub id: Option<Uuid>,
    pub entry_date: NaiveDate,
    #[validate(range(min = 1, max = 5))]
    pub mood_score: Option<u8>,
    pub tags: Option<Vec<String>>,
    #[validate(length(min = 1))]
    pub salt_base64: String,
    #[validate(length(min = 1))]
    pub nonce_base64: String,
    #[validate(length(min = 1))]
    pub ciphertext_base64: String,
    /// Explicitly forbidden: any plaintext content will trigger a 400 Bad Request
    pub content: Option<serde_json::Value>,
}

/// Full encrypted response payload returned to clients for local decryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedJournalResponse {
    pub id: Uuid,
    pub entry_date: NaiveDate,
    pub mood_score: Option<u8>,
    pub tags: Vec<String>,
    pub salt_base64: String,
    pub nonce_base64: String,
    pub ciphertext_base64: String,
}

/// Handler for `GET /api/v1/journal/metadata`
///
/// Returns chronological journal metadata omitting heavy ciphertext BLOBs.
///
/// # Errors
/// Returns [`ApiError::Internal`] if repository queries fail.
pub async fn list_metadata(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Query(query): Query<JournalMetadataQuery>,
) -> Result<Json<Vec<JournalMetadata>>, ApiError> {
    let repo = SqliteJournalRepository::new(state.db);
    let end_date = query.end_date.unwrap_or_else(|| Utc::now().date_naive());
    let start_date = query
        .start_date
        .unwrap_or_else(|| end_date - Duration::days(365));

    let metadata_list = repo
        .list_metadata_in_range(start_date, end_date)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(metadata_list))
}

/// Handler for `GET /api/v1/journal/{id}`
///
/// Fetches the complete encrypted envelope for client-side decryption.
///
/// # Errors
/// Returns [`ApiError::NotFound`] if entry does not exist, or [`ApiError::Internal`] on db error.
pub async fn get_entry(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EncryptedJournalResponse>, ApiError> {
    let repo = SqliteJournalRepository::new(state.db);
    let entry_opt = repo
        .get_entry_by_id(id)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let Some(entry) = entry_opt else {
        return Err(ApiError::NotFound(format!("Journal entry {id} not found")));
    };

    Ok(Json(EncryptedJournalResponse {
        id: entry.metadata.id,
        entry_date: entry.metadata.entry_date,
        mood_score: entry.metadata.mood_score,
        tags: entry.metadata.tags,
        salt_base64: BASE64_STANDARD.encode(entry.salt),
        nonce_base64: BASE64_STANDARD.encode(entry.nonce),
        ciphertext_base64: BASE64_STANDARD.encode(entry.encrypted_payload),
    }))
}

/// Handler for `POST /api/v1/journal`
///
/// Saves or updates a pre-encrypted envelope. Server never receives plaintext.
///
/// # Errors
/// Returns [`ApiError::BadRequest`] if input validation fails, malformed Base64 is sent,
/// or a zero-knowledge violation occurs.
pub async fn save_entry(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(payload): Json<SaveEncryptedJournalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.content.is_some() {
        return Err(ApiError::BadRequest(
            "Zero-knowledge violation: server rejects plaintext content. Reflections must be encrypted client-side."
                .to_string(),
        ));
    }

    payload
        .validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Decode and validate cryptographic artifacts
    let salt = BASE64_STANDARD
        .decode(&payload.salt_base64)
        .map_err(|e| ApiError::BadRequest(format!("Invalid salt Base64: {e}")))?;
    if salt.len() != 16 {
        return Err(ApiError::BadRequest(
            "Salt must be exactly 16 bytes".to_string(),
        ));
    }

    let nonce = BASE64_STANDARD
        .decode(&payload.nonce_base64)
        .map_err(|e| ApiError::BadRequest(format!("Invalid nonce Base64: {e}")))?;
    if nonce.len() != 12 {
        return Err(ApiError::BadRequest(
            "Nonce must be exactly 12 bytes".to_string(),
        ));
    }

    let ciphertext = BASE64_STANDARD
        .decode(&payload.ciphertext_base64)
        .map_err(|e| ApiError::BadRequest(format!("Invalid ciphertext Base64: {e}")))?;
    if ciphertext.len() < 16 {
        return Err(ApiError::BadRequest(
            "Ciphertext must be at least 16 bytes for AES-256-GCM authentication tag".to_string(),
        ));
    }

    let mut salt_arr = [0u8; 16];
    salt_arr.copy_from_slice(&salt);

    let mut nonce_arr = [0u8; 12];
    nonce_arr.copy_from_slice(&nonce);

    let repo = SqliteJournalRepository::new(state.db);

    // Reuse existing entry ID if saving for the same date without specifying ID
    let entry_id = if let Some(explicit_id) = payload.id {
        explicit_id
    } else if let Ok(Some(existing)) = repo.get_entry_by_date(payload.entry_date).await {
        existing.metadata.id
    } else {
        Uuid::new_v4()
    };

    let tags = payload.tags.unwrap_or_default();
    let ciphertext_len = ciphertext.len();
    let metadata = JournalMetadata {
        id: entry_id,
        entry_date: payload.entry_date,
        mood_score: payload.mood_score,
        tags: tags.clone(),
        ciphertext_bytes_len: ciphertext_len,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let entry = EncryptedJournalEntry::new(metadata, salt_arr, nonce_arr, ciphertext)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    repo.save_entry(&entry)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok((
        StatusCode::CREATED,
        Json(EncryptedJournalResponse {
            id: entry_id,
            entry_date: payload.entry_date,
            mood_score: payload.mood_score,
            tags,
            salt_base64: payload.salt_base64,
            nonce_base64: payload.nonce_base64,
            ciphertext_base64: payload.ciphertext_base64,
        }),
    ))
}

/// Handler for `DELETE /api/v1/journal/{id}`
///
/// # Errors
/// Returns [`ApiError::NotFound`] if entry does not exist, or [`ApiError::Internal`] on db error.
pub async fn delete_entry(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = SqliteJournalRepository::new(state.db);
    match repo.delete_entry(id).await {
        Ok(()) => Ok((StatusCode::OK, Json(serde_json::json!({ "deleted": true })))),
        Err(JournalRepositoryError::NotFound(_)) => {
            Err(ApiError::NotFound(format!("Journal entry {id} not found")))
        }
        Err(e) => Err(ApiError::Internal(e.into())),
    }
}

/// Constructs router for journal endpoints
pub fn journal_routes() -> Router<AppState> {
    Router::new()
        .route("/metadata", get(list_metadata))
        .route("/", post(save_entry))
        .route("/{id}", get(get_entry).delete(delete_entry))
}
