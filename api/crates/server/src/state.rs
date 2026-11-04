use std::{collections::HashMap, fmt, sync::Arc, time::Instant};

use roubit_core::{
    AppConfig, DatabasePool, HabitRepository, JournalRepository, OkrRepository, PgHabitRepository,
    PgJournalRepository, PgOkrRepository, PgRoutineRepository, PgUserRepository, RoutineRepository,
    SqliteHabitRepository, SqliteJournalRepository, SqliteOkrRepository, SqliteRoutineRepository,
    SqliteUserRepository, UserRepository,
};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

/// Shared application state injected into Axum route handlers
#[derive(Clone)]
pub struct AppState {
    /// Dual-engine database connection pool (SQLite for local dev, PostgreSQL for prod/test)
    pub db: DatabasePool,
    /// Dynamic user repository trait object
    pub user_repo: Arc<dyn UserRepository>,
    /// Dynamic routine repository trait object
    pub routine_repo: Arc<dyn RoutineRepository>,
    /// Dynamic habit repository trait object
    pub habit_repo: Arc<dyn HabitRepository>,
    /// Dynamic OKR repository trait object
    pub okr_repo: Arc<dyn OkrRepository>,
    /// Dynamic encrypted journal repository trait object
    pub journal_repo: Arc<dyn JournalRepository>,
    /// Application runtime configuration
    pub config: AppConfig,
    /// Optional Redis service for distributed locks and caching
    pub redis: Option<roubit_core::redis::RedisService>,
    /// Broadcast channel for real-time reminder notifications
    pub tx_notification: tokio::sync::broadcast::Sender<roubit_core::ReminderNotification>,
    /// Fast L1 in-memory habit dashboard summary cache: (`cached_at`, `serialized_json`)
    pub summary_cache: Arc<RwLock<HashMap<uuid::Uuid, (Instant, String)>>>,
    /// In-memory token-bucket rate limiter protecting auth and API endpoints
    pub rate_limiter: Arc<crate::middleware::RateLimiter>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("db", &self.db)
            .field("config", &self.config)
            .field("redis", &self.redis)
            .finish_non_exhaustive()
    }
}

impl AppState {
    /// Creates a new [`AppState`] backed by a SQLite connection pool (convenience constructor for local dev and tests).
    #[must_use]
    pub fn new(db: SqlitePool, config: AppConfig) -> Self {
        Self::from_pool(DatabasePool::Sqlite(db), config)
    }

    /// Creates a new [`AppState`] dynamically wiring either SQLite or PostgreSQL domain repositories from a [`DatabasePool`].
    #[must_use]
    pub fn from_pool(db: DatabasePool, config: AppConfig) -> Self {
        let (tx_notification, _) = tokio::sync::broadcast::channel(1024);
        let summary_cache = Arc::new(RwLock::new(HashMap::new()));
        let rate_limiter = Arc::new(crate::middleware::RateLimiter::new(
            config.auth_rate_limit,
            config.general_rate_limit,
        ));

        match &db {
            DatabasePool::Sqlite(pool) => Self {
                user_repo: Arc::new(SqliteUserRepository::new(pool.clone())),
                routine_repo: Arc::new(SqliteRoutineRepository::new(pool.clone())),
                habit_repo: Arc::new(SqliteHabitRepository::new(pool.clone())),
                okr_repo: Arc::new(SqliteOkrRepository::new(pool.clone())),
                journal_repo: Arc::new(SqliteJournalRepository::new(pool.clone())),
                db,
                config,
                redis: None,
                tx_notification,
                summary_cache,
                rate_limiter,
            },
            DatabasePool::Postgres(pool) => Self {
                user_repo: Arc::new(PgUserRepository::new(pool.clone())),
                routine_repo: Arc::new(PgRoutineRepository::new(pool.clone())),
                habit_repo: Arc::new(PgHabitRepository::new(pool.clone())),
                okr_repo: Arc::new(PgOkrRepository::new(pool.clone())),
                journal_repo: Arc::new(PgJournalRepository::new(pool.clone())),
                db,
                config,
                redis: None,
                tx_notification,
                summary_cache,
                rate_limiter,
            },
        }
    }

    /// Attaches a [`roubit_core::redis::RedisService`] instance to the application state.
    #[must_use]
    pub fn with_redis(mut self, redis: roubit_core::redis::RedisService) -> Self {
        self.redis = Some(redis);
        self
    }

    /// Overrides the rate limiter with a custom instance (useful for unit and benchmark tests).
    #[must_use]
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<crate::middleware::RateLimiter>) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }
}
