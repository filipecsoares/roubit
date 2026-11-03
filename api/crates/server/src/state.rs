use std::{collections::HashMap, sync::Arc, time::Instant};

use roubit_core::AppConfig;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

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
    /// Fast L1 in-memory habit dashboard summary cache: (`cached_at`, `serialized_json`)
    pub summary_cache: Arc<RwLock<HashMap<uuid::Uuid, (Instant, String)>>>,
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
            summary_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Attaches a [`roubit_core::redis::RedisService`] instance to the application state.
    #[must_use]
    pub fn with_redis(mut self, redis: roubit_core::redis::RedisService) -> Self {
        self.redis = Some(redis);
        self
    }
}
