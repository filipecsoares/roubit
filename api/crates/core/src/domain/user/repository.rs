//! User Repository and SQLite / PostgreSQL Persistence (RF05)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, SqlitePool};
use uuid::Uuid;

use super::models::User;

/// Errors occurring during user repository operations.
#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    /// Internal database error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// User already exists with the given email address.
    #[error("User already exists with email: {0}")]
    AlreadyExists(String),

    /// User entity was not found.
    #[error("User not found")]
    NotFound,
}

/// Abstract contract for user persistence operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Retrieves a user by their email address.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepositoryError>;

    /// Retrieves a user by their unique identifier.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError>;

    /// Persists a new user record.
    async fn create_user(&self, user: &User) -> Result<(), UserRepositoryError>;
}

/// SQLite-backed implementation of [`UserRepository`].
#[derive(Debug, Clone)]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    /// Creates a new `SqliteUserRepository` wrapping an active connection pool.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying `SQLx` SQLite connection pool.
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepositoryError> {
        let row =
            sqlx::query("SELECT id, email, password_hash, created_at FROM users WHERE email = ?")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let id: Uuid = row.try_get("id").or_else(|_| {
            row.try_get::<String, _>("id")
                .and_then(|s| Uuid::parse_str(&s).map_err(|e| sqlx::Error::Decode(Box::new(e))))
        })?;
        let email: String = row.get("email");
        let password_hash: String = row.get("password_hash");
        let created_at: String = row.get("created_at");

        Ok(Some(User::new(id, email, password_hash, created_at)))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError> {
        let row =
            sqlx::query("SELECT id, email, password_hash, created_at FROM users WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let email: String = row.get("email");
        let password_hash: String = row.get("password_hash");
        let created_at: String = row.get("created_at");

        Ok(Some(User::new(id, email, password_hash, created_at)))
    }

    async fn create_user(&self, user: &User) -> Result<(), UserRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO users (id, email, password_hash, created_at)
            VALUES (?, ?, ?, ?)
            ",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.created_at)
        .execute(&self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.is_unique_violation() {
                    return UserRepositoryError::AlreadyExists(user.email.clone());
                }
            }
            UserRepositoryError::Database(err)
        })?;

        Ok(())
    }
}

/// PostgreSQL-backed implementation of [`UserRepository`].
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// Creates a new `PgUserRepository` wrapping an active PostgreSQL connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Accesses the underlying `SQLx` PostgreSQL connection pool.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    fn map_row(row: &sqlx::postgres::PgRow) -> Result<User, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let email: String = row.try_get("email")?;
        let password_hash: String = row.try_get("password_hash")?;
        let created_at = row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map(|dt| dt.to_rfc3339())
            .or_else(|_| row.try_get::<String, _>("created_at"))?;

        Ok(User::new(id, email, password_hash, created_at))
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepositoryError> {
        let row =
            sqlx::query("SELECT id, email, password_hash, created_at FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;

        row.as_ref()
            .map(Self::map_row)
            .transpose()
            .map_err(UserRepositoryError::Database)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserRepositoryError> {
        let row =
            sqlx::query("SELECT id, email, password_hash, created_at FROM users WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        row.as_ref()
            .map(Self::map_row)
            .transpose()
            .map_err(UserRepositoryError::Database)
    }

    async fn create_user(&self, user: &User) -> Result<(), UserRepositoryError> {
        let created_at_dt = DateTime::parse_from_rfc3339(&user.created_at)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));

        sqlx::query(
            r"
            INSERT INTO users (id, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4)
            ",
        )
        .bind(user.id.to_string())
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(created_at_dt)
        .execute(&self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(db_err) = &err {
                if db_err.is_unique_violation() || db_err.code().as_deref() == Some("23505") {
                    return UserRepositoryError::AlreadyExists(user.email.clone());
                }
            }
            UserRepositoryError::Database(err)
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::create_sqlite_pool;

    async fn setup_repo() -> SqliteUserRepository {
        let pool = create_sqlite_pool("sqlite::memory:")
            .await
            .expect("in-memory pool succeeds");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations succeed");

        SqliteUserRepository::new(pool)
    }

    #[tokio::test]
    async fn test_user_repository_crud_and_duplicate() {
        let repo = setup_repo().await;
        let user_id = Uuid::new_v4();
        let user = User::new(
            user_id,
            "alice@example.com".to_string(),
            "hashed_secret".to_string(),
            chrono::Utc::now().to_rfc3339(),
        );

        // Create user
        repo.create_user(&user).await.expect("create user succeeds");

        // Find by email
        let fetched_by_email = repo
            .find_by_email("alice@example.com")
            .await
            .expect("find by email succeeds")
            .expect("user found");
        assert_eq!(fetched_by_email.id, user_id);
        assert_eq!(fetched_by_email.email, "alice@example.com");
        assert_eq!(fetched_by_email.password_hash, "hashed_secret");

        // Find by id
        let fetched_by_id = repo
            .find_by_id(user_id)
            .await
            .expect("find by id succeeds")
            .expect("user found");
        assert_eq!(fetched_by_id.id, user_id);

        // Duplicate email rejection
        let duplicate = User::new(
            Uuid::new_v4(),
            "alice@example.com".to_string(),
            "other_secret".to_string(),
            chrono::Utc::now().to_rfc3339(),
        );
        let err = repo.create_user(&duplicate).await.unwrap_err();
        assert!(matches!(err, UserRepositoryError::AlreadyExists(_)));

        // Not found cases
        assert!(repo
            .find_by_email("unknown@example.com")
            .await
            .unwrap()
            .is_none());
        assert!(repo.find_by_id(Uuid::new_v4()).await.unwrap().is_none());
    }
}
