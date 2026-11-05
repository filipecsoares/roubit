//! Remote REST API client adapter with Idempotency-Key headers and Bearer auth.

use async_trait::async_trait;
use base64::prelude::*;
use chrono::NaiveDate;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use roubit_core::domain::{
    habit::models::{Habit, HabitLog},
    journal::models::{EncryptedJournalEntry, JournalMetadata},
    routine::models::{Routine, TimeBlock},
    routine::CurrentRoutineProgress,
    BlockStatus, KeyResult, Objective, ObjectiveWithKeyResults,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::{ClientError, RoubitClient};

/// Client implementation connecting remotely to an Axum REST API over HTTP/HTTPS.
#[derive(Debug, Clone)]
pub struct HttpClient {
    base_url: String,
    auth_token: Option<String>,
    client: Client,
}

impl HttpClient {
    /// Creates a new `HttpClient` targeting a remote API base URL.
    ///
    /// If `auth_token` is omitted, attempts to load credentials from the local config file.
    #[must_use]
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let trimmed_url = base_url.into().trim_end_matches('/').to_string();

        Self {
            base_url: trimmed_url,
            auth_token,
            client: Client::builder().build().unwrap_or_default(),
        }
    }

    /// Accesses the target base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Accesses the optional active authentication token.
    #[must_use]
    pub fn auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    fn build_headers(&self, idempotency_key: Option<Uuid>) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(token) = &self.auth_token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }

        if let Some(key) = idempotency_key {
            if let Ok(header_name) = HeaderName::from_bytes(b"idempotency-key") {
                if let Ok(header_val) = HeaderValue::from_str(&key.to_string()) {
                    headers.insert(header_name, header_val);
                }
            }
        }

        headers
    }

    async fn handle_response<T: DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, ClientError> {
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>().await.map_err(ClientError::from)
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(ClientError::Api {
                status: 401,
                message: "Unauthorized: Session is invalid or expired. Please run 'roubit auth login' to authenticate.".to_string(),
            })
        } else {
            let text = resp.text().await.unwrap_or_default();
            let clean_message = Self::extract_error_detail(&text);
            Err(ClientError::Api {
                status: status.as_u16(),
                message: clean_message,
            })
        }
    }

    async fn handle_empty_response(resp: reqwest::Response) -> Result<(), ClientError> {
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(ClientError::Api {
                status: 401,
                message: "Unauthorized: Session is invalid or expired. Please run 'roubit auth login' to authenticate.".to_string(),
            })
        } else {
            let text = resp.text().await.unwrap_or_default();
            let clean_message = Self::extract_error_detail(&text);
            Err(ClientError::Api {
                status: status.as_u16(),
                message: clean_message,
            })
        }
    }

    fn extract_error_detail(raw: &str) -> String {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(raw) {
            if let Some(detail) = json_val.get("detail").and_then(|d| d.as_str()) {
                return detail.to_string();
            }
            if let Some(msg) = json_val.get("message").and_then(|m| m.as_str()) {
                return msg.to_string();
            }
            if let Some(err) = json_val.get("error").and_then(|e| e.as_str()) {
                return err.to_string();
            }
        }
        if raw.trim().is_empty() {
            "Unknown API error".to_string()
        } else {
            raw.to_string()
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum HabitResponseItem {
    Nested {
        habit: Habit,
        #[allow(dead_code)]
        streak: roubit_core::StreakMetrics,
        #[allow(dead_code)]
        #[serde(default)]
        today_value: Option<f64>,
    },
    Flat {
        id: Uuid,
        user_id: Option<Uuid>,
        name: String,
        description: Option<String>,
        habit_type: roubit_core::HabitType,
        target_value: f64,
        unit: Option<String>,
        frequency_days: Option<u8>,
        color_hex: Option<String>,
        is_archived: Option<bool>,
        current_streak: Option<i32>,
        longest_streak: Option<i32>,
        created_at: Option<chrono::DateTime<chrono::Utc>>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    },
}

impl HabitResponseItem {
    fn into_habit(self) -> Habit {
        match self {
            Self::Nested { habit, .. } => habit,
            Self::Flat {
                id,
                user_id,
                name,
                description,
                habit_type,
                target_value,
                unit,
                frequency_days,
                color_hex,
                is_archived,
                current_streak,
                longest_streak,
                created_at,
                updated_at,
            } => Habit {
                id,
                user_id,
                name,
                description,
                habit_type,
                target_value,
                unit,
                frequency_days: frequency_days.unwrap_or(127),
                color_hex: color_hex.unwrap_or_else(|| "#4F46E5".to_string()),
                is_archived: is_archived.unwrap_or(false),
                current_streak: current_streak.unwrap_or(0),
                longest_streak: longest_streak.unwrap_or(0),
                created_at: created_at.unwrap_or_else(chrono::Utc::now),
                updated_at: updated_at.unwrap_or_else(chrono::Utc::now),
            },
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RoutineWithProgressDto {
    routine: Option<Routine>,
    #[serde(default)]
    blocks: Vec<TimeBlock>,
    #[serde(default)]
    progress: Option<CurrentRoutineProgress>,
    #[serde(default)]
    block_statuses: std::collections::HashMap<Uuid, BlockStatus>,
    #[serde(default)]
    completed_block_ids: Vec<Uuid>,
    #[serde(default)]
    skipped_block_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct EncryptedJournalResponseDto {
    id: Uuid,
    entry_date: NaiveDate,
    mood_score: Option<u8>,
    #[serde(default)]
    tags: Vec<String>,
    salt_base64: String,
    nonce_base64: String,
    ciphertext_base64: String,
}

impl EncryptedJournalResponseDto {
    fn try_into_encrypted_entry(self) -> Result<EncryptedJournalEntry, ClientError> {
        let salt = BASE64_STANDARD
            .decode(&self.salt_base64)
            .map_err(|e| ClientError::Local(format!("invalid salt base64: {e}")))?;
        let nonce = BASE64_STANDARD
            .decode(&self.nonce_base64)
            .map_err(|e| ClientError::Local(format!("invalid nonce base64: {e}")))?;
        let ciphertext = BASE64_STANDARD
            .decode(&self.ciphertext_base64)
            .map_err(|e| ClientError::Local(format!("invalid ciphertext base64: {e}")))?;

        let salt_arr: [u8; 16] = salt
            .try_into()
            .map_err(|_| ClientError::Local("salt must be 16 bytes".into()))?;
        let nonce_arr: [u8; 12] = nonce
            .try_into()
            .map_err(|_| ClientError::Local("nonce must be 12 bytes".into()))?;

        let metadata = roubit_core::domain::journal::models::JournalMetadata {
            id: self.id,
            user_id: None,
            entry_date: self.entry_date,
            mood_score: self.mood_score,
            tags: self.tags,
            ciphertext_bytes_len: ciphertext.len(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        EncryptedJournalEntry::new(metadata, salt_arr, nonce_arr, ciphertext)
            .map_err(|e| ClientError::Local(e.to_string()))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum ObjectiveResponseDto {
    WithKrs(ObjectiveWithKeyResults),
    Plain(Objective),
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CheckinResponseDto {
    log: HabitLog,
    #[allow(dead_code)]
    streak: roubit_core::StreakMetrics,
}

#[async_trait]
impl RoubitClient for HttpClient {
    async fn list_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let url = format!("{}/api/v1/habits", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        let items: Vec<HabitResponseItem> = Self::handle_response(resp).await?;
        Ok(items
            .into_iter()
            .map(HabitResponseItem::into_habit)
            .collect())
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let url = format!("{}/api/v1/habits", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        let items: Vec<HabitResponseItem> = Self::handle_response(resp).await?;
        Ok(items
            .into_iter()
            .map(HabitResponseItem::into_habit)
            .filter(|h| !h.is_archived)
            .collect())
    }

    async fn habit_checkin(
        &self,
        habit_id: Uuid,
        date: NaiveDate,
        value: f64,
    ) -> Result<HabitLog, ClientError> {
        let url = format!("{}/api/v1/habits/{habit_id}/checkin", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "date": date.to_string(),
            "value": value
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        let res: CheckinResponseDto = Self::handle_response(resp).await?;
        Ok(res.log)
    }

    async fn create_habit(
        &self,
        name: &str,
        habit_type: roubit_core::HabitType,
        target_value: f64,
        unit: Option<&str>,
        description: Option<&str>,
    ) -> Result<Habit, ClientError> {
        let url = format!("{}/api/v1/habits", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "name": name,
            "description": description,
            "habit_type": match habit_type {
                roubit_core::HabitType::Numeric => "numeric",
                roubit_core::HabitType::Timer => "timer",
                roubit_core::HabitType::Binary => "binary",
            },
            "target_value": target_value,
            "unit": unit,
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        Self::handle_response(resp).await
    }

    async fn archive_habit(&self, habit_id: Uuid) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/habits/{habit_id}/archive", self.base_url);
        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(Uuid::new_v4())))
            .send()
            .await?;
        Self::handle_empty_response(resp).await
    }

    async fn delete_habit(&self, habit_id: Uuid) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/habits/{habit_id}", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .headers(self.build_headers(Some(Uuid::new_v4())))
            .send()
            .await?;
        Self::handle_empty_response(resp).await
    }

    async fn get_active_routine(&self) -> Result<Option<Routine>, ClientError> {
        let url = format!("{}/api/v1/routines", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let res: RoutineWithProgressDto = Self::handle_response(resp).await?;
        Ok(res.routine)
    }

    async fn list_routine_blocks(&self, routine_id: Uuid) -> Result<Vec<TimeBlock>, ClientError> {
        let url = format!("{}/api/v1/routines/{routine_id}/blocks", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            let active_url = format!("{}/api/v1/routines", self.base_url);
            let active_resp = self
                .client
                .get(&active_url)
                .headers(self.build_headers(None))
                .send()
                .await?;
            if let Ok(res) = Self::handle_response::<RoutineWithProgressDto>(active_resp).await {
                if res.routine.as_ref().map(|r| r.id) == Some(routine_id) {
                    return Ok(res.blocks);
                }
            }
            return Ok(Vec::new());
        }

        Self::handle_response(resp).await
    }

    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
    ) -> Result<(), ClientError> {
        let url = format!(
            "{}/api/v1/routines/blocks/{block_id}/complete",
            self.base_url
        );
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "date": date.to_string(),
            "status": status.as_str()
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        Self::handle_empty_response(resp).await
    }

    async fn list_all_objectives(&self) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let url = format!("{}/api/v1/okrs", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, ClientError> {
        let url = format!("{}/api/v1/okrs?quarter={quarter}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn list_journal_metadata(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, ClientError> {
        let url = format!(
            "{}/api/v1/journal/metadata?start_date={from}&end_date={to}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn save_journal(
        &self,
        date: NaiveDate,
        ciphertext_base64: &str,
        salt_base64: &str,
        nonce_base64: &str,
        mood: Option<i32>,
        tags: &[String],
    ) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/journal", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "entry_date": date.to_string(),
            "ciphertext_base64": ciphertext_base64,
            "salt_base64": salt_base64,
            "nonce_base64": nonce_base64,
            "mood_score": mood.and_then(|m| u8::try_from(m).ok()),
            "tags": tags,
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        Self::handle_empty_response(resp).await
    }

    async fn get_active_routine_progress(
        &self,
        date: NaiveDate,
    ) -> Result<
        Option<(
            Routine,
            Vec<(TimeBlock, BlockStatus)>,
            CurrentRoutineProgress,
        )>,
        ClientError,
    > {
        let url = format!("{}/api/v1/routines", self.base_url);
        let mut headers = self.build_headers(None);
        if let Ok(val) = HeaderValue::from_str(&date.to_string()) {
            headers.insert(HeaderName::from_static("x-local-date"), val);
        }
        let now_time = chrono::Local::now().time();
        if let Ok(val) = HeaderValue::from_str(&now_time.format("%H:%M:%S").to_string()) {
            headers.insert(HeaderName::from_static("x-local-time"), val);
        }

        let resp = self.client.get(&url).headers(headers).send().await?;

        let dto: RoutineWithProgressDto = Self::handle_response(resp).await?;
        let Some(routine) = dto.routine else {
            return Ok(None);
        };

        let block_statuses: Vec<(TimeBlock, BlockStatus)> = dto
            .blocks
            .iter()
            .map(|b| {
                let status = dto.block_statuses.get(&b.id).copied().unwrap_or_else(|| {
                    if dto.completed_block_ids.contains(&b.id) {
                        BlockStatus::Completed
                    } else if dto.skipped_block_ids.contains(&b.id) {
                        BlockStatus::Skipped
                    } else {
                        BlockStatus::Pending
                    }
                });
                (b.clone(), status)
            })
            .collect();

        let progress = dto.progress.unwrap_or_else(|| {
            roubit_core::RoutineProgressTracker::compute_progress(
                &dto.blocks,
                &block_statuses,
                now_time,
            )
        });

        Ok(Some((routine, block_statuses, progress)))
    }

    async fn create_routine(
        &self,
        name: &str,
        blocks: &[TimeBlock],
        is_active: bool,
    ) -> Result<Routine, ClientError> {
        let url = format!("{}/api/v1/routines", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let block_dtos: Vec<serde_json::Value> = blocks
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                serde_json::json!({
                    "name": b.name,
                    "start_time": b.start_time.format("%H:%M").to_string(),
                    "end_time": b.end_time.format("%H:%M").to_string(),
                    "category": b.category,
                    "color_hex": b.color_hex,
                    "order_index": i32::try_from(idx).unwrap_or(0)
                })
            })
            .collect();

        let payload = serde_json::json!({
            "name": name,
            "description": null,
            "is_active": is_active,
            "blocks": block_dtos
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        Self::handle_response(resp).await
    }

    async fn activate_routine(&self, routine_id: Uuid) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/routines/{routine_id}/activate", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .send()
            .await?;

        Self::handle_empty_response(resp).await
    }

    async fn delete_routine(&self, routine_id: Uuid) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/routines/{routine_id}", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        Self::handle_empty_response(resp).await
    }

    async fn create_objective(
        &self,
        title: &str,
        quarter: &str,
        description: Option<&str>,
    ) -> Result<Objective, ClientError> {
        let url = format!("{}/api/v1/okrs", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "title": title,
            "quarter": quarter,
            "description": description
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        let res: ObjectiveResponseDto = Self::handle_response(resp).await?;
        let objective = match res {
            ObjectiveResponseDto::WithKrs(w) => w.objective,
            ObjectiveResponseDto::Plain(o) => o,
        };
        Ok(objective)
    }

    async fn create_key_result(
        &self,
        objective_id: Uuid,
        title: &str,
        target_value: f64,
        unit: Option<&str>,
        linked_habit_id: Option<Uuid>,
    ) -> Result<KeyResult, ClientError> {
        let url = format!("{}/api/v1/okrs/{objective_id}/key-results", self.base_url);
        let idempotency_key = Uuid::new_v4();

        let payload = serde_json::json!({
            "title": title,
            "target_value": target_value,
            "unit": unit.unwrap_or("%"),
            "linked_habit_id": linked_habit_id
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.build_headers(Some(idempotency_key)))
            .json(&payload)
            .send()
            .await?;

        Self::handle_response(resp).await
    }

    async fn delete_objective(&self, objective_id: Uuid) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/okrs/{objective_id}", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        Self::handle_empty_response(resp).await
    }

    async fn get_journal_entry(
        &self,
        id: Uuid,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError> {
        let url = format!("{}/api/v1/journal/{id}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let dto: EncryptedJournalResponseDto = Self::handle_response(resp).await?;
        dto.try_into_encrypted_entry().map(Some)
    }

    async fn get_journal_entry_by_date(
        &self,
        date: NaiveDate,
    ) -> Result<Option<EncryptedJournalEntry>, ClientError> {
        let url = format!("{}/api/v1/journal/by-date/{date}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let dto: EncryptedJournalResponseDto = Self::handle_response(resp).await?;
        dto.try_into_encrypted_entry().map(Some)
    }

    async fn delete_journal_entry_by_date(&self, date: NaiveDate) -> Result<bool, ClientError> {
        let url = format!("{}/api/v1/journal/by-date/{date}", self.base_url);
        let resp = self
            .client
            .delete(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(false)
        } else {
            Self::handle_empty_response(resp).await?;
            Ok(true)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_build_headers_injects_bearer_and_idempotency_key() {
        let client = HttpClient::new("http://127.0.0.1:8080", Some("my-secret-jwt".to_string()));
        let key = Uuid::new_v4();

        let headers = client.build_headers(Some(key));
        assert_eq!(
            headers.get("authorization").unwrap().to_str().unwrap(),
            "Bearer my-secret-jwt"
        );
        assert_eq!(
            headers.get("idempotency-key").unwrap().to_str().unwrap(),
            key.to_string()
        );
    }

    #[test]
    fn test_build_headers_without_auth_or_key() {
        let client = HttpClient::new("http://127.0.0.1:8080", None);
        let headers = client.build_headers(None);

        assert!(!headers.contains_key("authorization"));
        assert!(!headers.contains_key("idempotency-key"));
    }

    #[test]
    fn test_extract_error_detail_rfc7807_and_fallbacks() {
        let detail_json = r#"{"type":"about:blank","title":"Conflict","status":409,"detail":"Checkin already recorded"}"#;
        assert_eq!(
            HttpClient::extract_error_detail(detail_json),
            "Checkin already recorded"
        );

        let msg_json = r#"{"message":"Invalid habit payload"}"#;
        assert_eq!(
            HttpClient::extract_error_detail(msg_json),
            "Invalid habit payload"
        );

        let err_json = r#"{"error":"Resource not found"}"#;
        assert_eq!(
            HttpClient::extract_error_detail(err_json),
            "Resource not found"
        );

        let plain_txt = "Internal Server Error";
        assert_eq!(
            HttpClient::extract_error_detail(plain_txt),
            "Internal Server Error"
        );
    }

    #[test]
    fn test_habit_response_item_deserializes_both_nested_and_flat_formats() {
        // 1. Nested format from roubit-server list_habits
        let nested_json = serde_json::json!([{
            "habit": {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Morning Run",
                "description": "5km daily",
                "habit_type": "numeric",
                "target_value": 5.0,
                "unit": "km",
                "frequency_days": 127,
                "color_hex": "#4F46E5",
                "is_archived": false,
                "current_streak": 3,
                "longest_streak": 5,
                "created_at": "2026-11-05T00:00:00Z",
                "updated_at": "2026-11-05T00:00:00Z"
            },
            "streak": {
                "current_streak": 3,
                "longest_streak": 5,
                "is_active_today": true
            },
            "today_value": 5.0
        }]);

        let nested_items: Vec<HabitResponseItem> =
            serde_json::from_value(nested_json).expect("Nested habit deserialization");
        assert_eq!(nested_items.len(), 1);
        let habit1 = nested_items.into_iter().next().unwrap().into_habit();
        assert_eq!(habit1.name, "Morning Run");
        assert_eq!(habit1.target_value, 5.0);

        // 2. Flat format
        let flat_json = serde_json::json!([{
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "name": "Read Book",
            "description": "20 pages",
            "habit_type": "binary",
            "target_value": 1.0,
            "unit": "pages",
            "frequency_days": 127,
            "color_hex": "#10B981",
            "current_streak": 2,
            "longest_streak": 10,
            "is_active_today": false
        }]);

        let flat_items: Vec<HabitResponseItem> =
            serde_json::from_value(flat_json).expect("Flat habit deserialization");
        assert_eq!(flat_items.len(), 1);
        let habit2 = flat_items.into_iter().next().unwrap().into_habit();
        assert_eq!(habit2.name, "Read Book");
        assert_eq!(habit2.target_value, 1.0);
    }

    #[test]
    fn test_routine_with_progress_dto_deserialization() {
        let routine_json = serde_json::json!({
            "routine": {
                "id": "550e8400-e29b-41d4-a716-446655440002",
                "name": "Daily Routine",
                "description": "Standard flow",
                "is_active": true,
                "created_at": "2026-11-05T00:00:00Z",
                "updated_at": "2026-11-05T00:00:00Z"
            },
            "blocks": [{
                "id": "550e8400-e29b-41d4-a716-446655440003",
                "routine_id": "550e8400-e29b-41d4-a716-446655440002",
                "name": "Deep Work",
                "start_time": "09:00:00",
                "end_time": "11:00:00",
                "category": "Focus",
                "color_hex": "#3B82F6",
                "order_index": 0,
                "created_at": "2026-11-05T00:00:00Z"
            }],
            "progress": null,
            "completed_block_ids": [],
            "skipped_block_ids": []
        });

        let res: RoutineWithProgressDto =
            serde_json::from_value(routine_json).expect("Routine with progress deserialization");
        assert!(res.routine.is_some());
        assert_eq!(res.blocks.len(), 1);
        assert_eq!(res.blocks[0].name, "Deep Work");
    }
}
