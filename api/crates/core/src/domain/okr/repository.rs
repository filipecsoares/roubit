//! Asynchronous SQLite and PostgreSQL repositories for Objectives and Key Results (RF03).

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, SqlitePool};
use uuid::Uuid;

use super::models::{KeyResult, Objective};

/// Errors encountered during OKR repository operations.
#[derive(Debug, thiserror::Error)]
pub enum OkrRepositoryError {
    /// Low-level database error from `SQLx`.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Target objective was not found.
    #[error("Objective not found: {0}")]
    ObjectiveNotFound(Uuid),

    /// Target key result was not found.
    #[error("Key Result not found: {0}")]
    KeyResultNotFound(Uuid),

    /// Error related to habit integration or retrieval.
    #[error("Habit repository error: {0}")]
    Habit(String),
}

/// A composite view combining an [`Objective`] with its child [`KeyResult`] items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveWithKeyResults {
    /// The parent objective.
    pub objective: Objective,
    /// Ordered list of key results belonging to this objective.
    pub key_results: Vec<KeyResult>,
}

/// Asynchronous repository contract for Objectives and Key Results.
#[async_trait]
pub trait OkrRepository: Send + Sync {
    /// Persists a new [`Objective`].
    async fn create_objective(
        &self,
        user_id: Uuid,
        objective: &Objective,
    ) -> Result<(), OkrRepositoryError>;

    /// Persists a new [`KeyResult`] and recalculates the parent objective's progress.
    async fn create_key_result(
        &self,
        user_id: Uuid,
        kr: &KeyResult,
    ) -> Result<(), OkrRepositoryError>;

    /// Fetches an objective by ID along with all its child key results.
    async fn get_objective_with_krs(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all objectives and their key results for a given planning quarter (e.g., "2026-Q4").
    async fn list_objectives_by_quarter(
        &self,
        user_id: Uuid,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all objectives across all quarters.
    async fn list_all_objectives(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all key results linked to a specific habit.
    async fn list_key_results_by_habit(
        &self,
        user_id: Uuid,
        habit_id: Uuid,
    ) -> Result<Vec<KeyResult>, OkrRepositoryError>;

    /// Updates the current achieved value of a [`KeyResult`], atomically updating both the
    /// Key Result's progress percentage and the parent Objective's weighted progress.
    async fn update_key_result_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        new_value: f64,
    ) -> Result<KeyResult, OkrRepositoryError>;

    /// Updates a Key Result's linked habit and/or current value, updating progress for both KR and parent Objective.
    async fn update_key_result_link_or_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        linked_habit_id: Option<Option<Uuid>>,
        new_value: Option<f64>,
    ) -> Result<KeyResult, OkrRepositoryError>;

    /// Explicitly updates an objective's progress percentage.
    async fn update_objective_progress(
        &self,
        user_id: Uuid,
        objective_id: Uuid,
        progress: f32,
    ) -> Result<(), OkrRepositoryError>;

    /// Deletes an objective and cascades deletion to all child key results.
    async fn delete_objective(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError>;

    /// Deletes a key result and recalculates the parent objective's progress.
    async fn delete_key_result(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError>;
}

/// SQLite-backed implementation of [`OkrRepository`].
#[derive(Debug, Clone)]
pub struct SqliteOkrRepository {
    pool: SqlitePool,
}

impl SqliteOkrRepository {
    /// Creates a new `SqliteOkrRepository` instance with the provided SQLite connection pool.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying [`SqlitePool`].
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl OkrRepository for SqliteOkrRepository {
    async fn create_objective(
        &self,
        user_id: Uuid,
        objective: &Objective,
    ) -> Result<(), OkrRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO objectives (id, user_id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(objective.id)
        .bind(user_id)
        .bind(&objective.title)
        .bind(&objective.description)
        .bind(&objective.quarter)
        .bind(objective.start_date)
        .bind(objective.end_date)
        .bind(objective.progress_percentage)
        .bind(objective.is_completed)
        .bind(objective.created_at)
        .bind(objective.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_key_result(
        &self,
        user_id: Uuid,
        kr: &KeyResult,
    ) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Verify parent objective exists and belongs to user_id
        let objective =
            sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ? AND user_id = ?")
                .bind(kr.objective_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let mut obj = objective.ok_or(OkrRepositoryError::ObjectiveNotFound(kr.objective_id))?;

        // 1b. If linked to a habit, verify the habit exists and belongs to user_id
        if let Some(habit_id) = kr.linked_habit_id {
            let habit_exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM habits WHERE id = ? AND user_id = ?")
                    .bind(habit_id)
                    .bind(user_id)
                    .fetch_optional(&mut *tx)
                    .await?;

            if habit_exists.is_none() {
                return Err(OkrRepositoryError::Habit(format!(
                    "Habit {habit_id} not found"
                )));
            }
        }

        // 2. Insert Key Result
        sqlx::query(
            r"
            INSERT INTO key_results (id, objective_id, linked_habit_id, title, initial_value, target_value, current_value, unit, weight, progress_percentage, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(kr.id)
        .bind(kr.objective_id)
        .bind(kr.linked_habit_id)
        .bind(&kr.title)
        .bind(kr.initial_value)
        .bind(kr.target_value)
        .bind(kr.current_value)
        .bind(&kr.unit)
        .bind(kr.weight)
        .bind(kr.progress_percentage)
        .bind(kr.created_at)
        .bind(kr.updated_at)
        .execute(&mut *tx)
        .await?;

        // 3. Fetch all KRs including newly inserted to recalculate parent progress
        let all_krs = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
        )
        .bind(kr.objective_id)
        .fetch_all(&mut *tx)
        .await?;

        obj.update_progress(&all_krs);

        sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = ?, is_completed = ?, updated_at = ?
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(obj.progress_percentage)
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_objective_with_krs(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objective =
            sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        let Some(obj) = objective else {
            return Ok(None);
        };

        let key_results = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(ObjectiveWithKeyResults {
            objective: obj,
            key_results,
        }))
    }

    async fn list_objectives_by_quarter(
        &self,
        user_id: Uuid,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objectives = sqlx::query_as::<_, Objective>(
            "SELECT * FROM objectives WHERE user_id = ? AND quarter = ? ORDER BY start_date ASC, created_at ASC",
        )
        .bind(user_id)
        .bind(quarter)
        .fetch_all(&self.pool)
        .await?;

        if objectives.is_empty() {
            return Ok(Vec::new());
        }

        let all_key_results = sqlx::query_as::<_, KeyResult>(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE o.user_id = ? AND o.quarter = ?
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(user_id)
        .bind(quarter)
        .fetch_all(&self.pool)
        .await?;

        let mut kr_map: HashMap<Uuid, Vec<KeyResult>> = HashMap::new();
        for kr in all_key_results {
            kr_map.entry(kr.objective_id).or_default().push(kr);
        }

        let results = objectives
            .into_iter()
            .map(|obj| {
                let key_results = kr_map.remove(&obj.id).unwrap_or_default();
                ObjectiveWithKeyResults {
                    objective: obj,
                    key_results,
                }
            })
            .collect();

        Ok(results)
    }

    async fn list_all_objectives(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objectives = sqlx::query_as::<_, Objective>(
            "SELECT * FROM objectives WHERE user_id = ? ORDER BY quarter DESC, start_date ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        if objectives.is_empty() {
            return Ok(Vec::new());
        }

        let all_key_results = sqlx::query_as::<_, KeyResult>(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE o.user_id = ?
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut kr_map: HashMap<Uuid, Vec<KeyResult>> = HashMap::new();
        for kr in all_key_results {
            kr_map.entry(kr.objective_id).or_default().push(kr);
        }

        let results = objectives
            .into_iter()
            .map(|obj| {
                let key_results = kr_map.remove(&obj.id).unwrap_or_default();
                ObjectiveWithKeyResults {
                    objective: obj,
                    key_results,
                }
            })
            .collect();

        Ok(results)
    }

    async fn list_key_results_by_habit(
        &self,
        user_id: Uuid,
        habit_id: Uuid,
    ) -> Result<Vec<KeyResult>, OkrRepositoryError> {
        let key_results = sqlx::query_as::<_, KeyResult>(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.linked_habit_id = ? AND o.user_id = ?
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(habit_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(key_results)
    }

    async fn update_key_result_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        new_value: f64,
    ) -> Result<KeyResult, OkrRepositoryError> {
        self.update_key_result_link_or_value(user_id, kr_id, None, Some(new_value))
            .await
    }

    async fn update_key_result_link_or_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        linked_habit_id: Option<Option<Uuid>>,
        new_value: Option<f64>,
    ) -> Result<KeyResult, OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch KeyResult and verify ownership via parent Objective
        let kr_opt = sqlx::query_as::<_, KeyResult>(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.id = ? AND o.user_id = ?
            ",
        )
        .bind(kr_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let mut kr = kr_opt.ok_or(OkrRepositoryError::KeyResultNotFound(kr_id))?;

        if let Some(link_opt) = linked_habit_id {
            kr.linked_habit_id = link_opt;
        }

        if let Some(val) = new_value {
            kr.update_current_value(val);
        } else {
            kr.updated_at = Utc::now();
        }

        sqlx::query(
            r"
            UPDATE key_results
            SET linked_habit_id = ?, current_value = ?, progress_percentage = ?, updated_at = ?
            WHERE id = ?
            ",
        )
        .bind(kr.linked_habit_id)
        .bind(kr.current_value)
        .bind(kr.progress_percentage)
        .bind(kr.updated_at)
        .bind(kr.id)
        .execute(&mut *tx)
        .await?;

        // 3. Fetch all siblings and recalculate parent Objective
        let obj_opt =
            sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ? AND user_id = ?")
                .bind(kr.objective_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let mut obj = obj_opt.ok_or(OkrRepositoryError::ObjectiveNotFound(kr.objective_id))?;

        let all_krs = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
        )
        .bind(kr.objective_id)
        .fetch_all(&mut *tx)
        .await?;

        obj.update_progress(&all_krs);

        sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = ?, is_completed = ?, updated_at = ?
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(obj.progress_percentage)
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(kr)
    }

    async fn update_objective_progress(
        &self,
        user_id: Uuid,
        objective_id: Uuid,
        progress: f32,
    ) -> Result<(), OkrRepositoryError> {
        let is_completed = progress >= 100.0;
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = ?, is_completed = ?, updated_at = ?
            WHERE id = ? AND user_id = ?
            ",
        )
        .bind(progress.clamp(0.0, 100.0))
        .bind(is_completed)
        .bind(now)
        .bind(objective_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(objective_id));
        }

        Ok(())
    }

    async fn delete_objective(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError> {
        let result = sqlx::query("DELETE FROM objectives WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(id));
        }

        Ok(())
    }

    async fn delete_key_result(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch KR and verify parent objective ownership
        let kr_opt = sqlx::query_as::<_, KeyResult>(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.id = ? AND o.user_id = ?
            ",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let kr = kr_opt.ok_or(OkrRepositoryError::KeyResultNotFound(id))?;

        // 2. Delete the KR
        sqlx::query("DELETE FROM key_results WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 3. Recalculate parent objective
        let obj_opt =
            sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ? AND user_id = ?")
                .bind(kr.objective_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some(mut obj) = obj_opt {
            let remaining_krs = sqlx::query_as::<_, KeyResult>(
                "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
            )
            .bind(kr.objective_id)
            .fetch_all(&mut *tx)
            .await?;

            obj.update_progress(&remaining_krs);

            sqlx::query(
                r"
                UPDATE objectives
                SET progress_percentage = ?, is_completed = ?, updated_at = ?
                WHERE id = ? AND user_id = ?
                ",
            )
            .bind(obj.progress_percentage)
            .bind(obj.is_completed)
            .bind(obj.updated_at)
            .bind(obj.id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

/// PostgreSQL-backed implementation of [`OkrRepository`].
#[derive(Debug, Clone)]
pub struct PgOkrRepository {
    pool: PgPool,
}

impl PgOkrRepository {
    /// Creates a new `PgOkrRepository` instance with the provided PostgreSQL connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying [`PgPool`].
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[allow(clippy::cast_possible_truncation)]
    fn map_objective_row(row: &sqlx::postgres::PgRow) -> Result<Objective, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let user_id_str: Option<String> = row.try_get("user_id").ok().flatten();
        let user_id = user_id_str.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        let title: String = row.try_get("title")?;
        let description: Option<String> = row.try_get("description")?;
        let quarter: String = row.try_get("quarter")?;
        let start_date: NaiveDate = row.try_get("start_date")?;
        let end_date: NaiveDate = row.try_get("end_date")?;
        let progress_f64: f64 = row.try_get("progress_percentage")?;
        let is_completed: bool = row.try_get("is_completed")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        Ok(Objective {
            id,
            user_id,
            title,
            description,
            quarter,
            start_date,
            end_date,
            progress_percentage: progress_f64 as f32,
            is_completed,
            created_at,
            updated_at,
        })
    }

    #[allow(clippy::cast_possible_truncation)]
    fn map_kr_row(row: &sqlx::postgres::PgRow) -> Result<KeyResult, sqlx::Error> {
        let id_str: String = row.try_get("id")?;
        let id = Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let objective_id_str: String = row.try_get("objective_id")?;
        let objective_id =
            Uuid::parse_str(&objective_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        let linked_habit_id_str: Option<String> = row.try_get("linked_habit_id").ok().flatten();
        let linked_habit_id = linked_habit_id_str
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());
        let title: String = row.try_get("title")?;
        let initial_value: f64 = row.try_get("initial_value")?;
        let target_value: f64 = row.try_get("target_value")?;
        let current_value: f64 = row.try_get("current_value")?;
        let unit: String = row.try_get("unit")?;
        let weight_f64: f64 = row.try_get("weight")?;
        let progress_f64: f64 = row.try_get("progress_percentage")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        Ok(KeyResult {
            id,
            objective_id,
            linked_habit_id,
            title,
            initial_value,
            target_value,
            current_value,
            unit,
            weight: weight_f64 as f32,
            progress_percentage: progress_f64 as f32,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl OkrRepository for PgOkrRepository {
    async fn create_objective(
        &self,
        user_id: Uuid,
        objective: &Objective,
    ) -> Result<(), OkrRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO objectives (id, user_id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ",
        )
        .bind(objective.id.to_string())
        .bind(user_id.to_string())
        .bind(&objective.title)
        .bind(&objective.description)
        .bind(&objective.quarter)
        .bind(objective.start_date)
        .bind(objective.end_date)
        .bind(f64::from(objective.progress_percentage))
        .bind(objective.is_completed)
        .bind(objective.created_at)
        .bind(objective.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_key_result(
        &self,
        user_id: Uuid,
        kr: &KeyResult,
    ) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        let objective_row = sqlx::query("SELECT * FROM objectives WHERE id = $1 AND user_id = $2")
            .bind(kr.objective_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;

        let obj_row =
            objective_row.ok_or(OkrRepositoryError::ObjectiveNotFound(kr.objective_id))?;
        let mut obj = Self::map_objective_row(&obj_row)?;

        if let Some(habit_id) = kr.linked_habit_id {
            let habit_exists: Option<String> =
                sqlx::query_scalar("SELECT id FROM habits WHERE id = $1 AND user_id = $2")
                    .bind(habit_id.to_string())
                    .bind(user_id.to_string())
                    .fetch_optional(&mut *tx)
                    .await?;

            if habit_exists.is_none() {
                return Err(OkrRepositoryError::Habit(format!(
                    "Habit {habit_id} not found"
                )));
            }
        }

        sqlx::query(
            r"
            INSERT INTO key_results (id, objective_id, linked_habit_id, title, initial_value, target_value, current_value, unit, weight, progress_percentage, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ",
        )
        .bind(kr.id.to_string())
        .bind(kr.objective_id.to_string())
        .bind(kr.linked_habit_id.map(|id| id.to_string()))
        .bind(&kr.title)
        .bind(kr.initial_value)
        .bind(kr.target_value)
        .bind(kr.current_value)
        .bind(&kr.unit)
        .bind(f64::from(kr.weight))
        .bind(f64::from(kr.progress_percentage))
        .bind(kr.created_at)
        .bind(kr.updated_at)
        .execute(&mut *tx)
        .await?;

        let kr_rows = sqlx::query(
            "SELECT * FROM key_results WHERE objective_id = $1 ORDER BY created_at ASC",
        )
        .bind(kr.objective_id.to_string())
        .fetch_all(&mut *tx)
        .await?;

        let all_krs = kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()?;

        obj.update_progress(&all_krs);

        sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = $1, is_completed = $2, updated_at = $3
            WHERE id = $4 AND user_id = $5
            ",
        )
        .bind(f64::from(obj.progress_percentage))
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id.to_string())
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_objective_with_krs(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objective_row = sqlx::query("SELECT * FROM objectives WHERE id = $1 AND user_id = $2")
            .bind(id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        let Some(obj_row) = objective_row else {
            return Ok(None);
        };
        let obj = Self::map_objective_row(&obj_row)?;

        let kr_rows = sqlx::query(
            "SELECT * FROM key_results WHERE objective_id = $1 ORDER BY created_at ASC",
        )
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let key_results = kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(ObjectiveWithKeyResults {
            objective: obj,
            key_results,
        }))
    }

    async fn list_objectives_by_quarter(
        &self,
        user_id: Uuid,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let obj_rows = sqlx::query(
            "SELECT * FROM objectives WHERE user_id = $1 AND quarter = $2 ORDER BY start_date ASC, created_at ASC",
        )
        .bind(user_id.to_string())
        .bind(quarter)
        .fetch_all(&self.pool)
        .await?;

        if obj_rows.is_empty() {
            return Ok(Vec::new());
        }

        let objectives = obj_rows
            .iter()
            .map(Self::map_objective_row)
            .collect::<Result<Vec<_>, _>>()?;

        let kr_rows = sqlx::query(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE o.user_id = $1 AND o.quarter = $2
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(user_id.to_string())
        .bind(quarter)
        .fetch_all(&self.pool)
        .await?;

        let all_key_results = kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()?;

        let mut kr_map: HashMap<Uuid, Vec<KeyResult>> = HashMap::new();
        for kr in all_key_results {
            kr_map.entry(kr.objective_id).or_default().push(kr);
        }

        let results = objectives
            .into_iter()
            .map(|obj| {
                let key_results = kr_map.remove(&obj.id).unwrap_or_default();
                ObjectiveWithKeyResults {
                    objective: obj,
                    key_results,
                }
            })
            .collect();

        Ok(results)
    }

    async fn list_all_objectives(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let obj_rows = sqlx::query(
            "SELECT * FROM objectives WHERE user_id = $1 ORDER BY quarter DESC, start_date ASC",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        if obj_rows.is_empty() {
            return Ok(Vec::new());
        }

        let objectives = obj_rows
            .iter()
            .map(Self::map_objective_row)
            .collect::<Result<Vec<_>, _>>()?;

        let kr_rows = sqlx::query(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE o.user_id = $1
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let all_key_results = kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()?;

        let mut kr_map: HashMap<Uuid, Vec<KeyResult>> = HashMap::new();
        for kr in all_key_results {
            kr_map.entry(kr.objective_id).or_default().push(kr);
        }

        let results = objectives
            .into_iter()
            .map(|obj| {
                let key_results = kr_map.remove(&obj.id).unwrap_or_default();
                ObjectiveWithKeyResults {
                    objective: obj,
                    key_results,
                }
            })
            .collect();

        Ok(results)
    }

    async fn list_key_results_by_habit(
        &self,
        user_id: Uuid,
        habit_id: Uuid,
    ) -> Result<Vec<KeyResult>, OkrRepositoryError> {
        let kr_rows = sqlx::query(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.linked_habit_id = $1 AND o.user_id = $2
            ORDER BY kr.created_at ASC
            ",
        )
        .bind(habit_id.to_string())
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(OkrRepositoryError::Database)
    }

    async fn update_key_result_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        new_value: f64,
    ) -> Result<KeyResult, OkrRepositoryError> {
        self.update_key_result_link_or_value(user_id, kr_id, None, Some(new_value))
            .await
    }

    async fn update_key_result_link_or_value(
        &self,
        user_id: Uuid,
        kr_id: Uuid,
        linked_habit_id: Option<Option<Uuid>>,
        new_value: Option<f64>,
    ) -> Result<KeyResult, OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        let kr_row_opt = sqlx::query(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.id = $1 AND o.user_id = $2
            ",
        )
        .bind(kr_id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        let kr_row = kr_row_opt.ok_or(OkrRepositoryError::KeyResultNotFound(kr_id))?;
        let mut kr = Self::map_kr_row(&kr_row)?;

        if let Some(link_opt) = linked_habit_id {
            kr.linked_habit_id = link_opt;
        }

        if let Some(val) = new_value {
            kr.update_current_value(val);
        } else {
            kr.updated_at = Utc::now();
        }

        let habit_id_str = kr.linked_habit_id.map(|id| id.to_string());

        sqlx::query(
            r"
            UPDATE key_results
            SET linked_habit_id = $1, current_value = $2, progress_percentage = $3, updated_at = $4
            WHERE id = $5
            ",
        )
        .bind(habit_id_str)
        .bind(kr.current_value)
        .bind(f64::from(kr.progress_percentage))
        .bind(kr.updated_at)
        .bind(kr.id.to_string())
        .execute(&mut *tx)
        .await?;

        let obj_row_opt = sqlx::query("SELECT * FROM objectives WHERE id = $1 AND user_id = $2")
            .bind(kr.objective_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;

        let obj_row = obj_row_opt.ok_or(OkrRepositoryError::ObjectiveNotFound(kr.objective_id))?;
        let mut obj = Self::map_objective_row(&obj_row)?;

        let kr_rows = sqlx::query(
            "SELECT * FROM key_results WHERE objective_id = $1 ORDER BY created_at ASC",
        )
        .bind(kr.objective_id.to_string())
        .fetch_all(&mut *tx)
        .await?;

        let all_krs = kr_rows
            .iter()
            .map(Self::map_kr_row)
            .collect::<Result<Vec<_>, _>>()?;

        obj.update_progress(&all_krs);

        sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = $1, is_completed = $2, updated_at = $3
            WHERE id = $4 AND user_id = $5
            ",
        )
        .bind(f64::from(obj.progress_percentage))
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id.to_string())
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(kr)
    }

    async fn update_objective_progress(
        &self,
        user_id: Uuid,
        objective_id: Uuid,
        progress: f32,
    ) -> Result<(), OkrRepositoryError> {
        let is_completed = progress >= 100.0;
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = $1, is_completed = $2, updated_at = $3
            WHERE id = $4 AND user_id = $5
            ",
        )
        .bind(f64::from(progress.clamp(0.0, 100.0)))
        .bind(is_completed)
        .bind(now)
        .bind(objective_id.to_string())
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(objective_id));
        }

        Ok(())
    }

    async fn delete_objective(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError> {
        let result = sqlx::query("DELETE FROM objectives WHERE id = $1 AND user_id = $2")
            .bind(id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(id));
        }

        Ok(())
    }

    async fn delete_key_result(&self, id: Uuid, user_id: Uuid) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        let kr_row_opt = sqlx::query(
            r"
            SELECT kr.id, kr.objective_id, kr.linked_habit_id, kr.title, kr.initial_value, kr.target_value, kr.current_value, kr.unit, kr.weight, kr.progress_percentage, kr.created_at, kr.updated_at
            FROM key_results kr
            JOIN objectives o ON kr.objective_id = o.id
            WHERE kr.id = $1 AND o.user_id = $2
            ",
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        let kr_row = kr_row_opt.ok_or(OkrRepositoryError::KeyResultNotFound(id))?;
        let kr = Self::map_kr_row(&kr_row)?;

        sqlx::query("DELETE FROM key_results WHERE id = $1")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;

        let obj_row_opt = sqlx::query("SELECT * FROM objectives WHERE id = $1 AND user_id = $2")
            .bind(kr.objective_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(obj_row) = obj_row_opt {
            let mut obj = Self::map_objective_row(&obj_row)?;
            let remaining_rows = sqlx::query(
                "SELECT * FROM key_results WHERE objective_id = $1 ORDER BY created_at ASC",
            )
            .bind(kr.objective_id.to_string())
            .fetch_all(&mut *tx)
            .await?;

            let remaining_krs = remaining_rows
                .iter()
                .map(Self::map_kr_row)
                .collect::<Result<Vec<_>, _>>()?;

            obj.update_progress(&remaining_krs);

            sqlx::query(
                r"
                UPDATE objectives
                SET progress_percentage = $1, is_completed = $2, updated_at = $3
                WHERE id = $4 AND user_id = $5
                ",
            )
            .bind(f64::from(obj.progress_percentage))
            .bind(obj.is_completed)
            .bind(obj.updated_at)
            .bind(obj.id.to_string())
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::similar_names)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_repo() -> SqliteOkrRepository {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteOkrRepository::new(pool)
    }

    #[tokio::test]
    async fn test_okr_creation_and_kr_association() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new(
            "Achieve Peak Fitness",
            Some("Q4 OKR".into()),
            "2026-Q4",
            start,
            end,
        )
        .with_user_id(user_id);
        repo.create_objective(user_id, &obj).await.unwrap();

        let kr = KeyResult::new_numeric(obj.id, "Run 150 km", 0.0, 150.0, "km", 1.0);
        repo.create_key_result(user_id, &kr).await.unwrap();

        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .expect("must exist");
        assert_eq!(loaded.objective.id, obj.id);
        assert_eq!(loaded.objective.user_id, Some(user_id));
        assert_eq!(loaded.key_results.len(), 1);
        assert_eq!(loaded.key_results[0].title, "Run 150 km");
    }

    #[tokio::test]
    async fn test_okr_multi_tenant_isolation() {
        let repo = setup_test_repo().await;
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_a)
            .await
            .unwrap();
        crate::domain::seed_test_user(repo.pool(), user_b)
            .await
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj_a =
            Objective::new("User A Objective", None, "2026-Q4", start, end).with_user_id(user_a);
        repo.create_objective(user_a, &obj_a).await.unwrap();

        let kr_a = KeyResult::new_numeric(obj_a.id, "Run 100 km", 0.0, 100.0, "km", 1.0);
        repo.create_key_result(user_a, &kr_a).await.unwrap();

        // User B cannot get Objective A
        assert!(repo
            .get_objective_with_krs(obj_a.id, user_b)
            .await
            .unwrap()
            .is_none());

        // User B's quarter list is empty
        assert_eq!(
            repo.list_objectives_by_quarter(user_b, "2026-Q4")
                .await
                .unwrap()
                .len(),
            0
        );

        // User B's all list is empty
        assert_eq!(repo.list_all_objectives(user_b).await.unwrap().len(), 0);

        // User B cannot add key result to Objective A
        let fake_kr = KeyResult::new_numeric(obj_a.id, "Malicious KR", 0.0, 10.0, "unit", 1.0);
        let add_res = repo.create_key_result(user_b, &fake_kr).await;
        assert!(matches!(
            add_res,
            Err(OkrRepositoryError::ObjectiveNotFound(_))
        ));

        // User B cannot update KR A
        let update_res = repo.update_key_result_value(user_b, kr_a.id, 50.0).await;
        assert!(matches!(
            update_res,
            Err(OkrRepositoryError::KeyResultNotFound(_))
        ));

        // User B cannot delete KR A
        let del_kr_res = repo.delete_key_result(kr_a.id, user_b).await;
        assert!(matches!(
            del_kr_res,
            Err(OkrRepositoryError::KeyResultNotFound(_))
        ));

        // User B cannot delete Objective A
        let del_obj_res = repo.delete_objective(obj_a.id, user_b).await;
        assert!(matches!(
            del_obj_res,
            Err(OkrRepositoryError::ObjectiveNotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_atomic_key_result_update_and_parent_recalculation() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Master Rust", None, "2026-Q4", start, end).with_user_id(user_id);
        repo.create_objective(user_id, &obj).await.unwrap();

        // KR 1: 100 pages read (weight 1.0)
        let kr1 = KeyResult::new_numeric(obj.id, "Read The Book", 0.0, 100.0, "pages", 1.0);
        repo.create_key_result(user_id, &kr1).await.unwrap();

        // KR 2: 10 Projects built (weight 1.0)
        let kr2 = KeyResult::new_numeric(obj.id, "Build Projects", 0.0, 10.0, "projects", 1.0);
        repo.create_key_result(user_id, &kr2).await.unwrap();

        // Parent starts at 0%
        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.objective.progress_percentage, 0.0);

        // Update KR 1 to 50 pages (50% progress for KR 1)
        // Parent progress: (50% * 1.0 + 0% * 1.0) / 2 = 25.0%
        let updated_kr1 = repo
            .update_key_result_value(user_id, kr1.id, 50.0)
            .await
            .unwrap();
        assert_eq!(updated_kr1.current_value, 50.0);
        assert_eq!(updated_kr1.progress_percentage, 50.0);

        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.objective.progress_percentage, 25.0);
        assert!(!loaded.objective.is_completed);

        // Update KR 2 to 10 projects (100% progress for KR 2)
        // Parent progress: (50% * 1.0 + 100% * 1.0) / 2 = 75.0%
        repo.update_key_result_value(user_id, kr2.id, 10.0)
            .await
            .unwrap();
        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.objective.progress_percentage, 75.0);

        // Update KR 1 to 100 pages (100%) -> Parent reaches 100% and is_completed = true!
        repo.update_key_result_value(user_id, kr1.id, 100.0)
            .await
            .unwrap();
        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.objective.progress_percentage, 100.0);
        assert!(loaded.objective.is_completed);
    }

    #[tokio::test]
    async fn test_list_objectives_by_quarter() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();

        let obj_q4 = Objective::new("Q4 Goal", None, "2026-Q4", start, end).with_user_id(user_id);
        let obj_annual =
            Objective::new("Annual Goal", None, "2026-Annual", start, end).with_user_id(user_id);

        repo.create_objective(user_id, &obj_q4).await.unwrap();
        repo.create_objective(user_id, &obj_annual).await.unwrap();

        let q4_list = repo
            .list_objectives_by_quarter(user_id, "2026-Q4")
            .await
            .unwrap();
        assert_eq!(q4_list.len(), 1);
        assert_eq!(q4_list[0].objective.title, "Q4 Goal");

        let all_list = repo.list_all_objectives(user_id).await.unwrap();
        assert_eq!(all_list.len(), 2);
    }

    #[tokio::test]
    async fn test_cascade_delete_objective_and_delete_key_result() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj =
            Objective::new("Temporary Goal", None, "2026-Q4", start, end).with_user_id(user_id);
        repo.create_objective(user_id, &obj).await.unwrap();

        let kr1 = KeyResult::new_numeric(obj.id, "KR 1", 0.0, 100.0, "units", 1.0);
        let kr2 = KeyResult::new_numeric(obj.id, "KR 2", 0.0, 100.0, "units", 1.0);
        repo.create_key_result(user_id, &kr1).await.unwrap();
        repo.create_key_result(user_id, &kr2).await.unwrap();

        // Delete KR 1
        repo.delete_key_result(kr1.id, user_id).await.unwrap();
        let loaded = repo
            .get_objective_with_krs(obj.id, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.key_results.len(), 1);
        assert_eq!(loaded.key_results[0].id, kr2.id);

        // Delete Objective (cascades to KR 2)
        repo.delete_objective(obj.id, user_id).await.unwrap();
        let loaded = repo.get_objective_with_krs(obj.id, user_id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_not_found_errors() {
        let repo = setup_test_repo().await;
        let user_id = Uuid::new_v4();
        crate::domain::seed_test_user(repo.pool(), user_id)
            .await
            .unwrap();
        let random_id = Uuid::new_v4();

        let res = repo.update_key_result_value(user_id, random_id, 10.0).await;
        assert!(matches!(res, Err(OkrRepositoryError::KeyResultNotFound(_))));

        let res = repo
            .update_objective_progress(user_id, random_id, 50.0)
            .await;
        assert!(matches!(res, Err(OkrRepositoryError::ObjectiveNotFound(_))));

        let res = repo.delete_objective(random_id, user_id).await;
        assert!(matches!(res, Err(OkrRepositoryError::ObjectiveNotFound(_))));
    }
}
