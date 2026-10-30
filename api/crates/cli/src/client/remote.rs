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

use super::{credentials::CredentialsManager, ClientError, RoubitClient};

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
        let token = auth_token.or_else(CredentialsManager::load_token);

        Self {
            base_url: trimmed_url,
            auth_token: token,
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
        } else {
            let message = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown API error".into());
            Err(ClientError::Api {
                status: status.as_u16(),
                message,
            })
        }
    }
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
        Self::handle_response(resp).await
    }

    async fn list_active_habits(&self) -> Result<Vec<Habit>, ClientError> {
        let url = format!("{}/api/v1/habits?active=true", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.build_headers(None))
            .send()
            .await?;
        Self::handle_response(resp).await
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

        Self::handle_response(resp).await
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

        if resp.status().is_success() {
            Ok(())
        } else {
            let status_code = resp.status().as_u16();
            let msg = resp.text().await.unwrap_or_default();
            Err(ClientError::Api {
                status: status_code,
                message: msg,
            })
        }
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
}
