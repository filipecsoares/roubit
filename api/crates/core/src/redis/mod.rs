//! Redis connection pooling, atomic distributed locks, and idempotency caching.

use deadpool_redis::redis::{AsyncCommands, RedisError as DriverRedisError, Script};
use deadpool_redis::{Config, CreatePoolError, Pool, PoolError, Runtime};
use thiserror::Error;

/// Error types related to Redis operations.
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("Redis pool error: {0}")]
    Pool(#[from] PoolError),

    #[error("Redis driver command error: {0}")]
    Command(#[from] DriverRedisError),

    #[error("Failed to create Redis pool: {0}")]
    CreatePool(#[from] CreatePoolError),

    #[error("Lock acquisition failed: resource is currently busy")]
    LockBusy,
}

/// Atomic Lua script for releasing a distributed lock safely:
/// Verifies that the lock value equals the token before deleting it.
const RELEASE_LOCK_LUA: &str = r#"
if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
else
    return 0
end
"#;

/// High-performance asynchronous Redis service with connection pooling.
#[derive(Clone, Debug)]
pub struct RedisService {
    pool: Pool,
}

impl RedisService {
    /// Creates a new `RedisService` by parsing a connection URL string.
    ///
    /// # Errors
    /// Returns `RedisError::CreatePool` if the URL is invalid or pool creation fails.
    pub fn new(redis_url: &str) -> Result<Self, RedisError> {
        let config = Config::from_url(redis_url);
        let pool = config.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self { pool })
    }

    /// Creates a `RedisService` wrapping an existing `deadpool_redis::Pool`.
    #[must_use]
    pub const fn from_pool(pool: Pool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying connection pool.
    #[must_use]
    pub const fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Acquires an asynchronous connection from the pool.
    ///
    /// # Errors
    /// Returns `RedisError::Pool` if retrieving a connection from the pool fails.
    pub async fn get_connection(&self) -> Result<deadpool_redis::Connection, RedisError> {
        Ok(self.pool.get().await?)
    }

    /// Pings the Redis instance to verify network health and responsiveness.
    ///
    /// # Errors
    /// Returns `RedisError` if the ping command fails.
    pub async fn ping(&self) -> Result<String, RedisError> {
        let mut conn = self.get_connection().await?;
        let res: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(res)
    }

    /// Acquires an atomic distributed lock with a millisecond TTL using `SET key token NX PX ttl_ms`.
    ///
    /// Returns `true` if lock was successfully obtained, or `false` if the resource is currently busy.
    ///
    /// # Errors
    /// Returns `RedisError` if the command fails.
    pub async fn acquire_lock(
        &self,
        resource_key: &str,
        lock_token: &str,
        ttl_ms: u64,
    ) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("lock:{resource_key}");

        let result: Option<String> = redis::cmd("SET")
            .arg(&full_key)
            .arg(lock_token)
            .arg("NX")
            .arg("PX")
            .arg(ttl_ms)
            .query_async(&mut conn)
            .await?;

        Ok(result.is_some())
    }

    /// Safely releases an atomic distributed lock using a Lua script to verify token ownership.
    ///
    /// Returns `true` if the lock was owned and released, `false` otherwise.
    ///
    /// # Errors
    /// Returns `RedisError` if executing the Lua script fails.
    pub async fn release_lock(
        &self,
        resource_key: &str,
        lock_token: &str,
    ) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("lock:{resource_key}");

        let script = Script::new(RELEASE_LOCK_LUA);
        let deleted_count: i64 = script
            .key(&full_key)
            .arg(lock_token)
            .invoke_async(&mut conn)
            .await?;

        Ok(deleted_count > 0)
    }

    /// Acquires an in-flight idempotency lock for mutations with a seconds TTL.
    /// Key format: `idempotency:lock:<user_id>:<key>`
    ///
    /// # Errors
    /// Returns `RedisError` if the command fails.
    pub async fn acquire_idempotency_lock(
        &self,
        user_id: &str,
        idempotency_key: &str,
        lock_token: &str,
        ttl_seconds: u64,
    ) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("idempotency:lock:{user_id}:{idempotency_key}");

        let result: Option<String> = redis::cmd("SET")
            .arg(&full_key)
            .arg(lock_token)
            .arg("NX")
            .arg("EX")
            .arg(ttl_seconds)
            .query_async(&mut conn)
            .await?;

        Ok(result.is_some())
    }

    /// Releases an in-flight idempotency lock using the token verification script.
    ///
    /// # Errors
    /// Returns `RedisError` if executing the Lua script fails.
    pub async fn release_idempotency_lock(
        &self,
        user_id: &str,
        idempotency_key: &str,
        lock_token: &str,
    ) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("idempotency:lock:{user_id}:{idempotency_key}");

        let script = Script::new(RELEASE_LOCK_LUA);
        let deleted_count: i64 = script
            .key(&full_key)
            .arg(lock_token)
            .invoke_async(&mut conn)
            .await?;

        Ok(deleted_count > 0)
    }

    /// Stores a cached idempotency response payload with a 24-hour expiration (`EX 86400`).
    /// Key format: `idempotency:response:<user_id>:<key>`
    ///
    /// # Errors
    /// Returns `RedisError` if the SETEX command fails.
    pub async fn cache_idempotency_response(
        &self,
        user_id: &str,
        idempotency_key: &str,
        response_bytes: &[u8],
        ttl_seconds: usize,
    ) -> Result<(), RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("idempotency:response:{user_id}:{idempotency_key}");

        conn.set_ex::<_, _, ()>(&full_key, response_bytes, ttl_seconds as u64)
            .await?;

        Ok(())
    }

    /// Retrieves a cached idempotency response payload if available.
    ///
    /// # Errors
    /// Returns `RedisError` if the GET command fails.
    pub async fn get_idempotency_response(
        &self,
        user_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<Vec<u8>>, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("idempotency:response:{user_id}:{idempotency_key}");

        let result: Option<Vec<u8>> = conn.get(&full_key).await?;
        Ok(result)
    }

    /// Schedules a reminder key in Redis with exact TTL (`fire_in_seconds`).
    /// Key format: `reminder:<user_id>:<entity_type>:<entity_id>`
    ///
    /// # Errors
    /// Returns `RedisError` if the SETEX command fails.
    pub async fn schedule_reminder(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        fire_in_seconds: usize,
    ) -> Result<(), RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("reminder:{user_id}:{entity_type}:{entity_id}");
        conn.set_ex::<_, _, ()>(&full_key, "active", fire_in_seconds as u64)
            .await?;
        Ok(())
    }

    /// Cancels a pending reminder key from Redis.
    ///
    /// # Errors
    /// Returns `RedisError` if the DEL command fails.
    pub async fn cancel_reminder(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let full_key = format!("reminder:{user_id}:{entity_type}:{entity_id}");
        let deleted: i64 = redis::cmd("DEL")
            .arg(&full_key)
            .query_async(&mut conn)
            .await?;
        Ok(deleted > 0)
    }
}

/// Notification payload broadcast when a scheduled reminder key expires.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReminderNotification {
    pub user_id: String,
    /// Entity type triggering the reminder ("`time_block`", "`habit`", etc.)
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub message: String,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
}

/// Helper scheduler for managing expiring reminder keys in Redis.
#[derive(Clone, Debug)]
pub struct ReminderScheduler {
    redis_service: std::sync::Arc<RedisService>,
}

impl ReminderScheduler {
    /// Creates a new `ReminderScheduler` wrapping a shared `RedisService`.
    #[must_use]
    pub const fn new(redis_service: std::sync::Arc<RedisService>) -> Self {
        Self { redis_service }
    }

    /// Schedules a reminder to fire after `fire_in_seconds`.
    ///
    /// # Errors
    /// Returns `RedisError` if scheduling fails.
    pub async fn schedule_reminder(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        fire_in_seconds: usize,
    ) -> Result<(), RedisError> {
        self.redis_service
            .schedule_reminder(user_id, entity_type, entity_id, fire_in_seconds)
            .await
    }

    /// Cancels a pending reminder.
    ///
    /// # Errors
    /// Returns `RedisError` if cancellation fails.
    pub async fn cancel_reminder(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<bool, RedisError> {
        self.redis_service
            .cancel_reminder(user_id, entity_type, entity_id)
            .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_service_new_valid_url() {
        let service = RedisService::new("redis://127.0.0.1:6379/0");
        assert!(service.is_ok());
    }

    #[test]
    fn test_redis_service_new_invalid_url() {
        let service = RedisService::new("http://invalid-redis-scheme");
        assert!(service.is_err());
    }

    #[test]
    fn test_redis_lua_script_content() {
        assert!(RELEASE_LOCK_LUA.contains("redis.call(\"get\", KEYS[1])"));
        assert!(RELEASE_LOCK_LUA.contains("redis.call(\"del\", KEYS[1])"));
    }

    #[test]
    fn test_redis_error_display() {
        let err = RedisError::LockBusy;
        assert_eq!(
            err.to_string(),
            "Lock acquisition failed: resource is currently busy"
        );
    }
}
