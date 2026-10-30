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
}

impl AppState {
    /// Creates a new [`AppState`] with the given database pool and configuration.
    pub const fn new(db: SqlitePool, config: AppConfig) -> Self {
        Self { db, config }
    }
}
