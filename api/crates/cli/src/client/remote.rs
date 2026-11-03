//! Remote REST API client adapter with Idempotency-Key headers and Bearer auth.

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use roubit_core::domain::{
    habit::models::{Habit, HabitLog},
    journal::models::JournalMetadata,
    routine::models::{Routine, TimeBlock},
    BlockStatus, ObjectiveWithKeyResults,
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
struct HabitWithStreakDto {
    habit: Habit,
    #[allow(dead_code)]
    streak: roubit_core::StreakMetrics,
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
        let items: Vec<HabitWithStreakDto> = Self::handle_response(resp).await?;
        Ok(items.into_iter().map(|item| item.habit).collect())
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let url = format!("{}/api/v1/habits?active=true", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        let items: Vec<HabitWithStreakDto> = Self::handle_response(resp).await?;
        Ok(items.into_iter().map(|item| item.habit).collect())
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
        let url = format!("{}/api/v1/routines/active", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        Self::handle_response(resp).await
    }

    async fn list_routine_blocks(&self, routine_id: Uuid) -> Result<Vec<TimeBlock>, ClientError> {
        let url = format!("{}/api/v1/routines/{routine_id}/blocks", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn log_block_status(
        &self,
        block_id: Uuid,
        date: NaiveDate,
        status: BlockStatus,
    ) -> Result<(), ClientError> {
        let url = format!("{}/api/v1/routines/blocks/{block_id}/log", self.base_url);
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
        let url = format!("{}/api/v1/journal?from={from}&to={to}", self.base_url);
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
            "mood": mood,
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
}
