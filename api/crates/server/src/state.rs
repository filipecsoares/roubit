//! Server Application State

use roubit_core::AppConfig;
use sqlx::SqlitePool;

/// Shared application state injected into Axum route handlers
#[derive(Clone, Debug)]
pub struct AppState {
    /// SQLite database connection pool
    pub db: SqlitePool,
    /// Application runtime configuration
    pub config: AppConfig,
    /// Optional Redis service for distributed locks and caching
    pub redis: Option<roubit_core::redis::RedisService>,
    /// Broadcast channel for real-time reminder notifications
    pub tx_notification: tokio::sync::broadcast::Sender<roubit_core::ReminderNotification>,
}

impl AppState {
    /// Creates a new [`AppState`] with the given database pool and configuration.
    #[must_use]
    pub fn new(db: SqlitePool, config: AppConfig) -> Self {
        let (tx_notification, _) = tokio::sync::broadcast::channel(1024);
        Self {
            db,
            config,
            redis: None,
            tx_notification,
        }
    }

    /// Attaches a [`roubit_core::redis::RedisService`] instance to the application state.
    #[must_use]
    pub fn with_redis(mut self, redis: roubit_core::redis::RedisService) -> Self {
        self.redis = Some(redis);
        self
    }
}
