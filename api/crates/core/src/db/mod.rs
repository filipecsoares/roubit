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
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(20)
                .min_connections(2)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .idle_timeout(std::time::Duration::from_secs(600))
                .connect(url)
                .await?;
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

    /// Executes a lightweight `SELECT 1` query against the active database pool to verify connectivity.
    ///
    /// # Errors
    /// Returns [`sqlx::Error`] if the database connection is unhealthy or unreachable.
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        match self {
            Self::Sqlite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            Self::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
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

/// Extracts a UUID string (`"xxxxxxxx-xxxx-..."`) from a SQLite row column that may be stored
/// either as a 16-byte binary `BLOB` (`sqlx::Uuid` default) or as a 36-char UTF-8 `TEXT` string.
///
/// # Errors
/// Returns [`DatabaseError::DataMigration`] if the column cannot be decoded as a valid UUID or non-empty identifier.
pub fn extract_uuid_str(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<String, DatabaseError> {
    if let Ok(u) = row.try_get::<uuid::Uuid, _>(col) {
        return Ok(u.to_string());
    }
    if let Ok(s) = row.try_get::<String, _>(col) {
        if let Ok(u) = uuid::Uuid::parse_str(&s) {
            return Ok(u.to_string());
        }
        if !s.is_empty() {
            return Ok(s);
        }
    }
    if let Ok(bytes) = row.try_get::<Vec<u8>, _>(col) {
        if let Ok(u) = uuid::Uuid::from_slice(&bytes) {
            return Ok(u.to_string());
        }
        if let Ok(s) = String::from_utf8(bytes) {
            if !s.is_empty() {
                return Ok(s);
            }
        }
    }
    Err(DatabaseError::DataMigration(format!(
        "Failed to decode UUID from SQLite column '{col}'"
    )))
}

/// Extracts an optional UUID string from a nullable SQLite column (`BLOB` or `TEXT`).
#[must_use]
pub fn extract_opt_uuid_str(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<String> {
    if let Ok(Some(u)) = row.try_get::<Option<uuid::Uuid>, _>(col) {
        return Some(u.to_string());
    }
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(col) {
        if let Ok(u) = uuid::Uuid::parse_str(&s) {
            return Some(u.to_string());
        }
        if !s.is_empty() {
            return Some(s);
        }
    }
    if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(col) {
        if let Ok(u) = uuid::Uuid::from_slice(&bytes) {
            return Some(u.to_string());
        }
        if let Ok(s) = String::from_utf8(bytes) {
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

/// Extracts a `DateTime<Utc>` from a SQLite column, supporting `SQLx`'s native SQLite datetime format
/// (`"YYYY-MM-DD HH:MM:SS.f UTC"`), RFC 3339 (`"YYYY-MM-DDTHH:MM:SSZ"`), and naive ISO strings.
///
/// # Errors
/// Returns [`DatabaseError::DataMigration`] if the timestamp cannot be decoded.
pub fn extract_datetime(
    row: &sqlx::sqlite::SqliteRow,
    col: &str,
) -> Result<chrono::DateTime<chrono::Utc>, DatabaseError> {
    if let Ok(dt) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(col) {
        return Ok(dt);
    }
    if let Ok(s) = row.try_get::<String, _>(col) {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
            return Ok(dt.with_timezone(&chrono::Utc));
        }
        let trimmed = s.trim_end_matches(" UTC").trim_end_matches('Z');
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%.f") {
            return Ok(ndt.and_utc());
        }
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(ndt.and_utc());
        }
    }
    Err(DatabaseError::DataMigration(format!(
        "Failed to decode DateTime<Utc> from SQLite column '{col}'"
    )))
}

/// Extracts an optional `DateTime<Utc>` from a nullable SQLite column.
#[must_use]
pub fn extract_opt_datetime(
    row: &sqlx::sqlite::SqliteRow,
    col: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(Some(dt)) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(col) {
        return Some(dt);
    }
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(col) {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
            return Some(dt.with_timezone(&chrono::Utc));
        }
        let trimmed = s.trim_end_matches(" UTC").trim_end_matches('Z');
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%.f") {
            return Some(ndt.and_utc());
        }
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(ndt.and_utc());
        }
    }
    None
}

/// Extracts a `NaiveDate` from a SQLite column.
///
/// # Errors
/// Returns [`DatabaseError::DataMigration`] if the date cannot be decoded.
pub fn extract_date(
    row: &sqlx::sqlite::SqliteRow,
    col: &str,
) -> Result<chrono::NaiveDate, DatabaseError> {
    if let Ok(d) = row.try_get::<chrono::NaiveDate, _>(col) {
        return Ok(d);
    }
    if let Ok(s) = row.try_get::<String, _>(col) {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
            return Ok(d);
        }
    }
    Err(DatabaseError::DataMigration(format!(
        "Failed to decode NaiveDate from SQLite column '{col}'"
    )))
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
    let default_user_id = "00000000-0000-0000-0000-000000000001".to_string();

    // 2. Migrate Users
    let users = sqlx::query("SELECT id, email, password_hash, created_at FROM users")
        .fetch_all(sqlite)
        .await?;

    for row in users {
        let id = extract_uuid_str(&row, "id")?;
        let email: String = row.try_get("email")?;
        let password_hash: String = row.try_get("password_hash")?;
        let parsed_created_at = extract_datetime(&row, "created_at")?;

        sqlx::query(
            r"
            INSERT INTO users (id, email, password_hash, created_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
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
        "SELECT id, user_id, name, description, is_active, created_at, updated_at FROM routines",
    )
    .fetch_all(sqlite)
    .await?;

    for row in routines {
        let id = extract_uuid_str(&row, "id")?;
        let user_id =
            extract_opt_uuid_str(&row, "user_id").unwrap_or_else(|| default_user_id.clone());
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let is_active = row
            .try_get::<bool, _>("is_active")
            .unwrap_or_else(|_| row.try_get::<i64, _>("is_active").unwrap_or(0) != 0);
        let p_created = extract_datetime(&row, "created_at")?;
        let p_updated = extract_datetime(&row, "updated_at")?;

        sqlx::query(
            r"
            INSERT INTO routines (id, user_id, name, description, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(user_id)
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
        let id = extract_uuid_str(&row, "id")?;
        let routine_id = extract_uuid_str(&row, "routine_id")?;
        let name: String = row.try_get("name")?;
        let start_time: String = row.try_get("start_time")?;
        let end_time: String = row.try_get("end_time")?;
        let category: String = row.try_get("category")?;
        let color_hex: String = row.try_get("color_hex")?;
        let order_index: i64 = row.try_get("order_index")?;
        let p_created = extract_datetime(&row, "created_at")?;
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
        let id = extract_uuid_str(&row, "id")?;
        let time_block_id = extract_uuid_str(&row, "time_block_id")?;
        let log_date = extract_date(&row, "date")?;
        let status: String = row.try_get("status")?;
        let p_started = extract_opt_datetime(&row, "started_at");
        let p_completed = extract_opt_datetime(&row, "completed_at");
        let notes: Option<String> = row.try_get("notes")?;

        sqlx::query(
            r"
            INSERT INTO daily_routine_logs (id, time_block_id, date, status, started_at, completed_at, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT DO NOTHING
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
        "SELECT id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at FROM habits",
    )
    .fetch_all(sqlite)
    .await?;

    for row in habits {
        let id = extract_uuid_str(&row, "id")?;
        let user_id =
            extract_opt_uuid_str(&row, "user_id").unwrap_or_else(|| default_user_id.clone());
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let habit_type: String = row.try_get("habit_type")?;
        let target_value: f64 = row.try_get("target_value")?;
        let unit: Option<String> = row.try_get("unit")?;
        let frequency_days: i64 = row.try_get("frequency_days")?;
        let color_hex: String = row.try_get("color_hex")?;
        let is_archived = row
            .try_get::<bool, _>("is_archived")
            .unwrap_or_else(|_| row.try_get::<i64, _>("is_archived").unwrap_or(0) != 0);
        let current_streak: i64 = row.try_get("current_streak")?;
        let longest_streak: i64 = row.try_get("longest_streak")?;
        let p_created = extract_datetime(&row, "created_at")?;
        let p_updated = extract_datetime(&row, "updated_at")?;

        let freq_i32 = i32::try_from(frequency_days).unwrap_or(127);
        let curr_i32 = i32::try_from(current_streak).unwrap_or(0);
        let long_i32 = i32::try_from(longest_streak).unwrap_or(0);

        sqlx::query(
            r"
            INSERT INTO habits (id, user_id, name, description, habit_type, target_value, unit, frequency_days, color_hex, is_archived, current_streak, longest_streak, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(habit_type)
        .bind(target_value)
        .bind(unit)
        .bind(freq_i32)
        .bind(color_hex)
        .bind(is_archived)
        .bind(curr_i32)
        .bind(long_i32)
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
        let id = extract_uuid_str(&row, "id")?;
        let habit_id = extract_uuid_str(&row, "habit_id")?;
        let log_date = extract_date(&row, "log_date")?;
        let actual_value: f64 = row.try_get("actual_value")?;
        let is_completed = row
            .try_get::<bool, _>("is_completed")
            .unwrap_or_else(|_| row.try_get::<i64, _>("is_completed").unwrap_or(0) != 0);
        let notes: Option<String> = row.try_get("notes")?;
        let p_logged = extract_datetime(&row, "logged_at")?;

        sqlx::query(
            r"
            INSERT INTO habit_logs (id, habit_id, log_date, actual_value, is_completed, notes, logged_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT DO NOTHING
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
        "SELECT id, user_id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at FROM objectives",
    )
    .fetch_all(sqlite)
    .await?;

    for row in objectives {
        let id = extract_uuid_str(&row, "id")?;
        let user_id =
            extract_opt_uuid_str(&row, "user_id").unwrap_or_else(|| default_user_id.clone());
        let title: String = row.try_get("title")?;
        let description: Option<String> = row.try_get("description")?;
        let quarter: String = row.try_get("quarter")?;
        let start_date = extract_date(&row, "start_date")?;
        let end_date = extract_date(&row, "end_date")?;
        let progress_percentage: f64 = row.try_get("progress_percentage")?;
        let is_completed = row
            .try_get::<bool, _>("is_completed")
            .unwrap_or_else(|_| row.try_get::<i64, _>("is_completed").unwrap_or(0) != 0);
        let p_created = extract_datetime(&row, "created_at")?;
        let p_updated = extract_datetime(&row, "updated_at")?;

        sqlx::query(
            r"
            INSERT INTO objectives (id, user_id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO NOTHING
            ",
        )
        .bind(id)
        .bind(user_id)
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
        let id = extract_uuid_str(&row, "id")?;
        let objective_id = extract_uuid_str(&row, "objective_id")?;
        let linked_habit_id = extract_opt_uuid_str(&row, "linked_habit_id");
        let title: String = row.try_get("title")?;
        let initial_value: f64 = row.try_get("initial_value")?;
        let target_value: f64 = row.try_get("target_value")?;
        let current_value: f64 = row.try_get("current_value")?;
        let unit: String = row.try_get("unit")?;
        let weight: f64 = row.try_get("weight")?;
        let progress_percentage: f64 = row.try_get("progress_percentage")?;
        let p_created = extract_datetime(&row, "created_at")?;
        let p_updated = extract_datetime(&row, "updated_at")?;

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
        "SELECT id, user_id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at FROM journal_entries",
    )
    .fetch_all(sqlite)
    .await?;

    for row in journal_entries {
        let id = extract_uuid_str(&row, "id")?;
        let user_id =
            extract_opt_uuid_str(&row, "user_id").unwrap_or_else(|| default_user_id.clone());
        let entry_date = extract_date(&row, "entry_date")?;
        let mood_score: Option<i64> = row.try_get("mood_score")?;
        let tags: String = row.try_get("tags")?;
        let salt: Vec<u8> = row.try_get("salt")?;
        let nonce: Vec<u8> = row.try_get("nonce")?;
        let encrypted_payload: Vec<u8> = row.try_get("encrypted_payload")?;
        let p_created = extract_datetime(&row, "created_at")?;
        let p_updated = extract_datetime(&row, "updated_at")?;
        let mood_i32 = mood_score.and_then(|m| i32::try_from(m).ok());

        sqlx::query(
            r"
            INSERT INTO journal_entries (id, user_id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT DO NOTHING
            ",
        )
        .bind(id)
        .bind(user_id)
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
        let key: String = row.try_get("key")?;
        let user_id = extract_uuid_str(&row, "user_id")?;
        let request_path: String = row.try_get("request_path")?;
        let request_hash: Vec<u8> = row.try_get("request_hash")?;
        let response_status: Option<i64> = row.try_get("response_status")?;
        let response_headers: Option<String> = row.try_get("response_headers")?;
        let response_body: Option<Vec<u8>> = row.try_get("response_body")?;
        let p_created = extract_datetime(&row, "created_at")?;
        let p_locked = extract_datetime(&row, "locked_until")?;
        let p_status = response_status.and_then(|s| i32::try_from(s).ok());

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

    #[tokio::test]
    async fn test_migration_0007_multi_tenancy_columns_and_fk_cascade() {
        let pool = crate::domain::create_sqlite_pool("sqlite::memory:")
            .await
            .unwrap();
        crate::run_migrations(&pool).await.unwrap();

        let user_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, password_hash, created_at) VALUES (?, 'user@test.com', 'hash', '2026-11-01T00:00:00Z')")
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        let habit_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO habits (id, user_id, name, habit_type, target_value, frequency_days, color_hex, is_archived, created_at, updated_at) VALUES (?, ?, 'Reading', 'binary', 1.0, 127, '#10B981', 0, '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z')")
            .bind(&habit_id)
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        let routine_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO routines (id, user_id, name, is_active, created_at, updated_at) VALUES (?, ?, 'Morning', 1, '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z')")
            .bind(&routine_id)
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        let objective_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO objectives (id, user_id, title, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at) VALUES (?, ?, 'Q4 Goals', '2026-Q4', '2026-10-01', '2026-12-31', 0.0, 0, '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z')")
            .bind(&objective_id)
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        let journal_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO journal_entries (id, user_id, entry_date, tags, salt, nonce, encrypted_payload, created_at, updated_at) VALUES (?, ?, '2026-11-01', '[]', X'01020304', X'05060708', X'090A0B0C', '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z')")
            .bind(&journal_id)
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        // Foreign key violation check with invalid user_id
        let invalid_habit_id = uuid::Uuid::new_v4().to_string();
        let invalid_user_id = uuid::Uuid::new_v4().to_string();
        let fk_err = sqlx::query("INSERT INTO habits (id, user_id, name, habit_type, target_value, frequency_days, color_hex, is_archived, created_at, updated_at) VALUES (?, ?, 'Fail Habit', 'binary', 1.0, 127, '#10B981', 0, '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z')")
            .bind(&invalid_habit_id)
            .bind(&invalid_user_id)
            .execute(&pool)
            .await;
        assert!(
            fk_err.is_err(),
            "Expected FK violation with invalid user_id"
        );

        // Verify Cascade deletion
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        let count_habits: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM habits WHERE id = ?")
            .bind(&habit_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count_habits.0, 0,
            "Habit should have been cascaded on user delete"
        );

        let count_routines: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM routines WHERE id = ?")
            .bind(&routine_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count_routines.0, 0,
            "Routine should have been cascaded on user delete"
        );

        let count_objectives: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM objectives WHERE id = ?")
                .bind(&objective_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count_objectives.0, 0,
            "Objective should have been cascaded on user delete"
        );

        let count_journal: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM journal_entries WHERE id = ?")
                .bind(&journal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count_journal.0, 0,
            "Journal entry should have been cascaded on user delete"
        );
    }

    #[tokio::test]
    async fn test_sqlite_row_extraction_helpers_blob_and_text() {
        let pool = crate::domain::create_sqlite_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE raw_test (
                blob_uuid BLOB NOT NULL,
                text_uuid TEXT NOT NULL,
                opt_uuid TEXT,
                sqlite_dt TEXT NOT NULL,
                rfc3339_dt TEXT NOT NULL,
                opt_dt TEXT,
                raw_date TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        let u1 = uuid::Uuid::new_v4();
        let u2 = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO raw_test (blob_uuid, text_uuid, opt_uuid, sqlite_dt, rfc3339_dt, opt_dt, raw_date)
             VALUES (?, ?, NULL, '2026-10-29 14:30:00.123 UTC', '2026-10-29T14:30:00Z', NULL, '2026-10-29')",
        )
        .bind(u1) // binds as 16-byte BLOB in SQLite
        .bind(u2.to_string()) // binds as 36-char TEXT in SQLite
        .execute(&pool)
        .await
        .unwrap();

        let row = sqlx::query("SELECT * FROM raw_test")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(extract_uuid_str(&row, "blob_uuid").unwrap(), u1.to_string());
        assert_eq!(extract_uuid_str(&row, "text_uuid").unwrap(), u2.to_string());
        assert_eq!(extract_opt_uuid_str(&row, "opt_uuid"), None);

        let dt1 = extract_datetime(&row, "sqlite_dt").unwrap();
        let dt2 = extract_datetime(&row, "rfc3339_dt").unwrap();
        assert_eq!(dt1.date_naive().to_string(), "2026-10-29");
        assert_eq!(dt2.to_rfc3339(), "2026-10-29T14:30:00+00:00");
        assert_eq!(extract_opt_datetime(&row, "opt_dt"), None);

        let d = extract_date(&row, "raw_date").unwrap();
        assert_eq!(d.to_string(), "2026-10-29");
    }
}
