//! Client factory for selecting between local SQLite engine and remote HTTP API.

use sqlx::SqlitePool;

use super::{ClientError, HttpClient, LocalClient, RoubitClient};

/// Execution mode of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMode {
    /// Direct embedded SQLite database execution.
    Local {
        /// Optional custom SQLite database path or URL.
        db_url: Option<String>,
    },
    /// Client-server execution via HTTP REST API.
    Remote {
        /// Remote API base URL (e.g., `<https://api.roubit.local>`).
        base_url: String,
        /// Optional JWT session token.
        auth_token: Option<String>,
    },
}

/// Factory responsible for instantiating the appropriate `RoubitClient`.
#[derive(Debug, Default)]
pub struct ClientFactory;

impl ClientFactory {
    /// Resolves the intended client mode based on CLI arguments and environment variables.
    #[must_use]
    #[allow(clippy::option_if_let_else)]
    pub fn resolve_mode(remote_flag: Option<&str>, db_flag: Option<&str>) -> ClientMode {
        if let Some(remote_url) = remote_flag.filter(|s| !s.trim().is_empty()) {
            let auth_token = std::env::var("ROUBIT_AUTH_TOKEN").ok();
            ClientMode::Remote {
                base_url: remote_url.to_string(),
                auth_token,
            }
        } else if let Ok(env_remote) = std::env::var("ROUBIT_REMOTE_URL") {
            let trimmed = env_remote.trim();
            if trimmed.is_empty() {
                ClientMode::Local {
                    db_url: db_flag.map(ToString::to_string),
                }
            } else {
                let auth_token = std::env::var("ROUBIT_AUTH_TOKEN").ok();
                ClientMode::Remote {
                    base_url: trimmed.to_string(),
                    auth_token,
                }
            }
        } else {
            ClientMode::Local {
                db_url: db_flag.map(ToString::to_string),
            }
        }
    }

    /// Creates an operational client instance (boxed trait object) matching the resolved mode.
    ///
    /// # Errors
    /// Returns `ClientError` if local pool initialization or migrations fail.
    pub async fn create_client(
        remote_flag: Option<&str>,
        db_flag: Option<&str>,
        fallback_pool: Option<SqlitePool>,
    ) -> Result<Box<dyn RoubitClient>, ClientError> {
        let mode = Self::resolve_mode(remote_flag, db_flag);
        match mode {
            ClientMode::Remote {
                base_url,
                auth_token,
            } => {
                let client = HttpClient::new(base_url, auth_token);
                Ok(Box::new(client))
            }
            ClientMode::Local { db_url } => {
                let pool = if let Some(p) = fallback_pool {
                    p
                } else {
                    let target_db = db_url.as_deref().unwrap_or("sqlite::memory:");
                    roubit_core::create_sqlite_pool(target_db)
                        .await
                        .map_err(|e| ClientError::Local(e.to_string()))?
                };
                roubit_core::run_migrations(&pool)
                    .await
                    .map_err(|e| ClientError::Local(e.to_string()))?;
                Ok(Box::new(LocalClient::new(pool)))
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_creates_local_client_by_default() {
        // Clear any lingering env variable for test isolation
        std::env::remove_var("ROUBIT_REMOTE_URL");
        let mode = ClientFactory::resolve_mode(None, Some("sqlite://local.db"));
        assert_eq!(
            mode,
            ClientMode::Local {
                db_url: Some("sqlite://local.db".to_string())
            }
        );
    }

    #[test]
    fn test_factory_creates_remote_client_when_remote_flag_provided() {
        let mode = ClientFactory::resolve_mode(Some("http://localhost:8080"), None);
        match mode {
            ClientMode::Remote { base_url, .. } => {
                assert_eq!(base_url, "http://localhost:8080");
            }
            ClientMode::Local { .. } => panic!("Expected ClientMode::Remote"),
        }
    }

    #[tokio::test]
    async fn test_factory_instantiates_local_client() {
        let client = ClientFactory::create_client(None, Some("sqlite::memory:"), None)
            .await
            .unwrap();

        let habits = client.list_habits().await.unwrap();
        assert_eq!(habits.len(), 0);
    }

    #[tokio::test]
    async fn test_factory_instantiates_remote_client() {
        let client = ClientFactory::create_client(Some("http://127.0.0.1:9999"), None, None)
            .await
            .unwrap();

        // Listing habits against inactive remote should produce a network error, proving HttpClient is active
        let result = client.list_habits().await;
        assert!(result.is_err());
    }
}
