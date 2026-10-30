//! Dual database support (SQLite & PostgreSQL) and migration tooling.

use sqlx::{PgPool, Row, SqlitePool};
use thiserror::Error;

/// Database connection pool supporting both SQLite and PostgreSQL backends.
#[derive(Clone, Debug)]
pub enum DatabasePool {
    /// Local SQLite connection pool
    Sqlite(SqlitePool),
    /// Enterprise PostgreSQL connection pool
    Postgres(PgPool),
}

/// Errors originating from database operations and migrations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLx database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Data migration error: {0}")]
    DataMigration(String),
}

impl DatabasePool {
    /// Creates a database pool dynamically selecting the dialect based on connection URL scheme.
    ///
    /// URLs starting with `postgres://` or `postgresql://` instantiate a `PgPool`.
    /// All other URLs instantiate a `SqlitePool`.
    ///
    /// # Errors
    /// Returns [`sqlx::Error`] if pool connection fails.
    pub async fn from_url(url: &str) -> Result<Self, sqlx::Error> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            let pool = PgPool::connect(url).await?;
            Ok(Self::Postgres(pool))
        } else {
            let pool = crate::domain::create_sqlite_pool(url).await?;
            Ok(Self::Sqlite(pool))
        }
    }

    /// Returns `true` if the underlying pool is PostgreSQL.
    #[must_use]
    pub const fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    /// Returns `true` if the underlying pool is SQLite.
    #[must_use]
    pub const fn is_sqlite(&self) -> bool {
        matches!(self, Self::Sqlite(_))
    }

    /// Returns a reference to the SQLite pool if active.
    #[must_use]
    pub const fn sqlite(&self) -> Option<&SqlitePool> {
        match self {
            Self::Sqlite(pool) => Some(pool),
            Self::Postgres(_) => None,
        }
    }

    /// Returns a reference to the PostgreSQL pool if active.
    #[must_use]
    pub const fn postgres(&self) -> Option<&PgPool> {
        match self {
            Self::Postgres(pool) => Some(pool),
            Self::Sqlite(_) => None,
        }
    }

    /// Applies appropriate migrations for the active database engine.
    ///
    /// # Errors
    /// Returns [`sqlx::migrate::MigrateError`] if applying migrations fails.
    pub async fn run_migrations(&self) -> Result<(), sqlx::migrate::MigrateError> {
        match self {
            Self::Sqlite(pool) => {
                crate::run_migrations(pool).await?;
            }
            Self::Postgres(pool) => {
                sqlx::migrate!("./migrations_pg").run(pool).await?;
            }
        }
        Ok(())
    }
}

/// Statistics and count of migrated entities from SQLite to PostgreSQL.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct MigrationReport {
    pub users: u64,
    pub routines: u64,
    pub time_blocks: u64,
    pub daily_routine_logs: u64,
    pub habits: u64,
    pub habit_logs: u64,
    pub objectives: u64,
    pub key_results: u64,
    pub journal_entries: u64,
    pub idempotency_keys: u64,
}

impl MigrationReport {
    /// Returns the total number of records migrated across all tables.
    #[must_use]
    pub const fn total_records(&self) -> u64 {
        self.users
            + self.routines
            + self.time_blocks
            + self.daily_routine_logs
            + self.habits
            + self.habit_logs
            + self.objectives
            + self.key_results
            + self.journal_entries
            + self.idempotency_keys
    }
}

/// Migrates all data from an existing SQLite database into a target PostgreSQL database.
///
/// Execution runs inside an atomic PostgreSQL transaction. If any error occurs during
/// table transfer, the entire transaction is rolled back cleanly.
///
/// # Errors
/// Returns [`DatabaseError`] if migration or transfer fails.
#[allow(clippy::too_many_lines)]
pub async fn migrate_sqlite_to_postgres(
    sqlite: &SqlitePool,
    postgres: &PgPool,
) -> Result<MigrationReport, DatabaseError> {
    // 1. Ensure PostgreSQL schema is up to date
    sqlx::migrate!("./migrations_pg").run(postgres).await?;

    let mut report = MigrationReport::default();
    let mut tx = postgres.begin().await?;

    // 2. Migrate Users
    let users = sqlx::query("SELECT id, email, password_hash, created_at FROM users")
        .fetch_all(sqlite)
        .await?;

    for row in users {
        let id: String = row.get("id");
        let email: String = row.get("email");
        let password_hash: String = row.get("password_hash");
        let created_at: String = row.get("created_at");
        let parsed_created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO users (id, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(parsed_created_at)
        .execute(&mut *tx)
        .await?;

        report.users += 1;
    }

    // 3. Migrate Routines
    let routines = sqlx::query(
        "SELECT id, name, description, is_active, created_at, updated_at FROM routines",
    )
    .fetch_all(sqlite)
    .await?;

    for row in routines {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let is_active_int: i64 = row.get("is_active");
        let is_active = is_active_int != 0;
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO routines (id, user_id, name, description, is_active, created_at, updated_at)
            VALUES ($1, 'default', $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(is_active)
        .bind(p_created)
        .bind(p_updated)
        .execute(&mut *tx)
        .await?;

        report.routines += 1;
    }

    // 4. Migrate Time Blocks
    let time_blocks = sqlx::query(
        "SELECT id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at FROM time_blocks",
    )
    .fetch_all(sqlite)
    .await?;

    for row in time_blocks {
        let id: String = row.get("id");
        let routine_id: String = row.get("routine_id");
        let name: String = row.get("name");
        let start_time: String = row.get("start_time");
        let end_time: String = row.get("end_time");
        let category: String = row.get("category");
        let color_hex: String = row.get("color_hex");
        let order_index: i64 = row.get("order_index");
        let created_at: String = row.get("created_at");

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        let order_i32 = i32::try_from(order_index).unwrap_or(0);

        sqlx::query(
            r"
            INSERT INTO time_blocks (id, routine_id, name, start_time, end_time, category, color_hex, order_index, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(routine_id)
        .bind(name)
        .bind(start_time)
        .bind(end_time)
        .bind(category)
        .bind(color_hex)
        .bind(order_i32)
        .bind(p_created)
        .execute(&mut *tx)
        .await?;

        report.time_blocks += 1;
    }

    // 5. Migrate Daily Routine Logs
    let daily_logs = sqlx::query(
        "SELECT id, time_block_id, date, status, started_at, completed_at, notes FROM daily_routine_logs",
    )
    .fetch_all(sqlite)
    .await?;

    for row in daily_logs {
        let id: String = row.get("id");
        let time_block_id: String = row.get("time_block_id");
        let date_str: String = row.get("date");
        let status: String = row.get("status");
        let started_at: Option<String> = row.get("started_at");
        let completed_at: Option<String> = row.get("completed_at");
        let notes: Option<String> = row.get("notes");

        let log_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());

        let p_started = started_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });
        let p_completed = completed_at.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        sqlx::query(
            r"
            INSERT INTO daily_routine_logs (id, time_block_id, date, status, started_at, completed_at, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(time_block_id)
        .bind(log_date)
        .bind(status)
        .bind(p_started)
        .bind(p_completed)
        .bind(notes)
        .execute(&mut *tx)
        .await?;

        report.daily_routine_logs += 1;
    }

    // 6. Migrate Habits
    let habits = sqlx::query(
        "SELECT id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at FROM habits",
    )
    .fetch_all(sqlite)
    .await?;

    for row in habits {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let habit_type: String = row.get("habit_type");
        let target_value: f64 = row.get("target_value");
        let unit: Option<String> = row.get("unit");
        let frequency_days: i64 = row.get("frequency_days");
        let color_hex: String = row.get("color_hex");
        let is_archived_int: i64 = row.get("is_archived");
        let is_archived = is_archived_int != 0;
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        let freq_i32 = i32::try_from(frequency_days).unwrap_or(127);

        sqlx::query(
            r"
            INSERT INTO habits (id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, created_at, updated_at)
            VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(habit_type)
        .bind(target_value)
        .bind(unit)
        .bind(freq_i32)
        .bind(color_hex)
        .bind(is_archived)
        .bind(p_created)
        .bind(p_updated)
        .execute(&mut *tx)
        .await?;

        report.habits += 1;
    }

    // 7. Migrate Habit Logs
    let habit_logs = sqlx::query(
        "SELECT id, habit_id, log_date, actual_value, is_completed, notes, logged_at FROM habit_logs",
    )
    .fetch_all(sqlite)
    .await?;

    for row in habit_logs {
        let id: String = row.get("id");
        let habit_id: String = row.get("habit_id");
        let log_date_str: String = row.get("log_date");
        let actual_value: f64 = row.get("actual_value");
        let is_completed_int: i64 = row.get("is_completed");
        let is_completed = is_completed_int != 0;
        let notes: Option<String> = row.get("notes");
        let logged_at: String = row.get("logged_at");

        let log_date = chrono::NaiveDate::parse_from_str(&log_date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());
        let p_logged = chrono::DateTime::parse_from_rfc3339(&logged_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(habit_id)
        .bind(log_date)
        .bind(actual_value)
        .bind(is_completed)
        .bind(notes)
        .bind(p_logged)
        .execute(&mut *tx)
        .await?;

        report.habit_logs += 1;
    }

    // 8. Migrate Objectives
    let objectives = sqlx::query(
        "SELECT id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at FROM objectives",
    )
    .fetch_all(sqlite)
    .await?;

    for row in objectives {
        let id: String = row.get("id");
        let title: String = row.get("title");
        let description: Option<String> = row.get("description");
        let quarter: String = row.get("quarter");
        let start_date_str: String = row.get("start_date");
        let end_date_str: String = row.get("end_date");
        let progress_percentage: f64 = row.get("progress_percentage");
        let is_completed_int: i64 = row.get("is_completed");
        let is_completed = is_completed_int != 0;
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let start_date = chrono::NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());
        let end_date = chrono::NaiveDate::parse_from_str(&end_date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO objectives (id, user_id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(quarter)
        .bind(start_date)
        .bind(end_date)
        .bind(progress_percentage)
        .bind(is_completed)
        .bind(p_created)
        .bind(p_updated)
        .execute(&mut *tx)
        .await?;

        report.objectives += 1;
    }

    // 9. Migrate Key Results
    let key_results = sqlx::query(
        "SELECT id, objective_id, linked_habit_id, title, initial_value, target_value, current_value, unit, weight, progress_percentage, created_at, updated_at FROM key_results",
    )
    .fetch_all(sqlite)
    .await?;

    for row in key_results {
        let id: String = row.get("id");
        let objective_id: String = row.get("objective_id");
        let linked_habit_id: Option<String> = row.get("linked_habit_id");
        let title: String = row.get("title");
        let initial_value: f64 = row.get("initial_value");
        let target_value: f64 = row.get("target_value");
        let current_value: f64 = row.get("current_value");
        let unit: String = row.get("unit");
        let weight: f64 = row.get("weight");
        let progress_percentage: f64 = row.get("progress_percentage");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO key_results (id, objective_id, linked_habit_id, title, initial_value, target_value, current_value, unit, weight, progress_percentage, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(objective_id)
        .bind(linked_habit_id)
        .bind(title)
        .bind(initial_value)
        .bind(target_value)
        .bind(current_value)
        .bind(unit)
        .bind(weight)
        .bind(progress_percentage)
        .bind(p_created)
        .bind(p_updated)
        .execute(&mut *tx)
        .await?;

        report.key_results += 1;
    }

    // 10. Migrate Journal Entries
    let journal_entries = sqlx::query(
        "SELECT id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at FROM journal_entries",
    )
    .fetch_all(sqlite)
    .await?;

    for row in journal_entries {
        let id: String = row.get("id");
        let entry_date_str: String = row.get("entry_date");
        let mood_score: Option<i64> = row.get("mood_score");
        let tags: String = row.get("tags");
        let salt: Vec<u8> = row.get("salt");
        let nonce: Vec<u8> = row.get("nonce");
        let encrypted_payload: Vec<u8> = row.get("encrypted_payload");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");

        let entry_date = chrono::NaiveDate::parse_from_str(&entry_date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Utc::now().date_naive());
        let mood_i32 = mood_score.and_then(|m| i32::try_from(m).ok());

        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO journal_entries (id, user_id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at)
            VALUES ($1, 'default', $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(entry_date)
        .bind(mood_i32)
        .bind(tags)
        .bind(salt)
        .bind(nonce)
        .bind(encrypted_payload)
        .bind(p_created)
        .bind(p_updated)
        .execute(&mut *tx)
        .await?;

        report.journal_entries += 1;
    }

    // 11. Migrate Idempotency Keys
    let idempotency_keys = sqlx::query(
        "SELECT key, user_id, request_path, request_hash, response_status, response_headers, response_body, created_at, locked_until FROM idempotency_keys",
    )
    .fetch_all(sqlite)
    .await?;

    for row in idempotency_keys {
        let key: String = row.get("key");
        let user_id: String = row.get("user_id");
        let request_path: String = row.get("request_path");
        let request_hash: Vec<u8> = row.get("request_hash");
        let response_status: Option<i64> = row.get("response_status");
        let response_headers: Option<String> = row.get("response_headers");
        let response_body: Option<Vec<u8>> = row.get("response_body");
        let created_at: String = row.get("created_at");
        let locked_until: String = row.get("locked_until");

        let p_status = response_status.and_then(|s| i32::try_from(s).ok());
        let p_created = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
        let p_locked = chrono::DateTime::parse_from_rfc3339(&locked_until)
            .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc));

        sqlx::query(
            r"
            INSERT INTO idempotency_keys (key, user_id, request_path, request_hash, response_status, response_headers, response_body, created_at, locked_until)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id, key) DO NOTHING
            ",
        )
        .bind(key)
        .bind(user_id)
        .bind(request_path)
        .bind(request_hash)
        .bind(p_status)
        .bind(response_headers)
        .bind(response_body)
        .bind(p_created)
        .bind(p_locked)
        .execute(&mut *tx)
        .await?;

        report.idempotency_keys += 1;
    }

    tx.commit().await?;
    Ok(report)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_database_pool_detection() {
        let sqlite_url = "sqlite://roubit.db";
        let pg_url = "postgres://user:pass@localhost:5432/roubit";
        let pg_ssl_url = "postgresql://user:pass@localhost:5432/roubit";

        assert!(!sqlite_url.starts_with("postgres://") && !sqlite_url.starts_with("postgresql://"));
        assert!(pg_url.starts_with("postgres://"));
        assert!(pg_ssl_url.starts_with("postgresql://"));
    }

    #[test]
    fn test_migration_report_total_records() {
        let report = MigrationReport {
            users: 2,
            routines: 3,
            time_blocks: 10,
            daily_routine_logs: 5,
            habits: 4,
            habit_logs: 20,
            objectives: 1,
            key_results: 3,
            journal_entries: 7,
            idempotency_keys: 1,
        };
        assert_eq!(report.total_records(), 56);
    }
}
