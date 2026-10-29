//! Asynchronous SQLite repository for Objectives and Key Results (RF03).

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
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
    async fn create_objective(&self, objective: &Objective) -> Result<(), OkrRepositoryError>;

    /// Persists a new [`KeyResult`] and recalculates the parent objective's progress.
    async fn create_key_result(&self, kr: &KeyResult) -> Result<(), OkrRepositoryError>;

    /// Fetches an objective by ID along with all its child key results.
    async fn get_objective_with_krs(
        &self,
        id: Uuid,
    ) -> Result<Option<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all objectives and their key results for a given planning quarter (e.g., "2026-Q4").
    async fn list_objectives_by_quarter(
        &self,
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all objectives across all quarters.
    async fn list_all_objectives(&self)
        -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError>;

    /// Lists all key results linked to a specific habit.
    async fn list_key_results_by_habit(
        &self,
        habit_id: Uuid,
    ) -> Result<Vec<KeyResult>, OkrRepositoryError>;

    /// Updates the current achieved value of a [`KeyResult`], atomically updating both the
    /// Key Result's progress percentage and the parent Objective's weighted progress.
    async fn update_key_result_value(
        &self,
        kr_id: Uuid,
        new_value: f64,
    ) -> Result<KeyResult, OkrRepositoryError>;

    /// Explicitly updates an objective's progress percentage.
    async fn update_objective_progress(
        &self,
        objective_id: Uuid,
        progress: f32,
    ) -> Result<(), OkrRepositoryError>;

    /// Deletes an objective and cascades deletion to all child key results.
    async fn delete_objective(&self, id: Uuid) -> Result<(), OkrRepositoryError>;

    /// Deletes a key result and recalculates the parent objective's progress.
    async fn delete_key_result(&self, id: Uuid) -> Result<(), OkrRepositoryError>;
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
    async fn create_objective(&self, objective: &Objective) -> Result<(), OkrRepositoryError> {
        sqlx::query(
            r"
            INSERT INTO objectives (id, title, description, quarter, start_date, end_date, progress_percentage, is_completed, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(objective.id)
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

    async fn create_key_result(&self, kr: &KeyResult) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Verify parent objective exists
        let objective = sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ?")
            .bind(kr.objective_id)
            .fetch_optional(&mut *tx)
            .await?;

        let mut obj = objective.ok_or(OkrRepositoryError::ObjectiveNotFound(kr.objective_id))?;

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
            WHERE id = ?
            ",
        )
        .bind(obj.progress_percentage)
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_objective_with_krs(
        &self,
        id: Uuid,
    ) -> Result<Option<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objective = sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ?")
            .bind(id)
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
        quarter: &str,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objectives = sqlx::query_as::<_, Objective>(
            "SELECT * FROM objectives WHERE quarter = ? ORDER BY start_date ASC, created_at ASC",
        )
        .bind(quarter)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(objectives.len());
        for obj in objectives {
            let key_results = sqlx::query_as::<_, KeyResult>(
                "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
            )
            .bind(obj.id)
            .fetch_all(&self.pool)
            .await?;

            results.push(ObjectiveWithKeyResults {
                objective: obj,
                key_results,
            });
        }

        Ok(results)
    }

    async fn list_all_objectives(
        &self,
    ) -> Result<Vec<ObjectiveWithKeyResults>, OkrRepositoryError> {
        let objectives = sqlx::query_as::<_, Objective>(
            "SELECT * FROM objectives ORDER BY quarter DESC, start_date ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::with_capacity(objectives.len());
        for obj in objectives {
            let key_results = sqlx::query_as::<_, KeyResult>(
                "SELECT * FROM key_results WHERE objective_id = ? ORDER BY created_at ASC",
            )
            .bind(obj.id)
            .fetch_all(&self.pool)
            .await?;

            results.push(ObjectiveWithKeyResults {
                objective: obj,
                key_results,
            });
        }

        Ok(results)
    }

    async fn list_key_results_by_habit(
        &self,
        habit_id: Uuid,
    ) -> Result<Vec<KeyResult>, OkrRepositoryError> {
        let key_results = sqlx::query_as::<_, KeyResult>(
            "SELECT * FROM key_results WHERE linked_habit_id = ? ORDER BY created_at ASC",
        )
        .bind(habit_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(key_results)
    }

    async fn update_key_result_value(
        &self,
        kr_id: Uuid,
        new_value: f64,
    ) -> Result<KeyResult, OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch KeyResult
        let kr_opt = sqlx::query_as::<_, KeyResult>("SELECT * FROM key_results WHERE id = ?")
            .bind(kr_id)
            .fetch_optional(&mut *tx)
            .await?;

        let mut kr = kr_opt.ok_or(OkrRepositoryError::KeyResultNotFound(kr_id))?;

        // 2. Update KR value and progress
        kr.update_current_value(new_value);

        sqlx::query(
            r"
            UPDATE key_results
            SET current_value = ?, progress_percentage = ?, updated_at = ?
            WHERE id = ?
            ",
        )
        .bind(kr.current_value)
        .bind(kr.progress_percentage)
        .bind(kr.updated_at)
        .bind(kr.id)
        .execute(&mut *tx)
        .await?;

        // 3. Fetch all siblings and recalculate parent Objective
        let obj_opt = sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ?")
            .bind(kr.objective_id)
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
            WHERE id = ?
            ",
        )
        .bind(obj.progress_percentage)
        .bind(obj.is_completed)
        .bind(obj.updated_at)
        .bind(obj.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(kr)
    }

    async fn update_objective_progress(
        &self,
        objective_id: Uuid,
        progress: f32,
    ) -> Result<(), OkrRepositoryError> {
        let is_completed = progress >= 100.0;
        let now = Utc::now();

        let result = sqlx::query(
            r"
            UPDATE objectives
            SET progress_percentage = ?, is_completed = ?, updated_at = ?
            WHERE id = ?
            ",
        )
        .bind(progress.clamp(0.0, 100.0))
        .bind(is_completed)
        .bind(now)
        .bind(objective_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(objective_id));
        }

        Ok(())
    }

    async fn delete_objective(&self, id: Uuid) -> Result<(), OkrRepositoryError> {
        let result = sqlx::query("DELETE FROM objectives WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(OkrRepositoryError::ObjectiveNotFound(id));
        }

        Ok(())
    }

    async fn delete_key_result(&self, id: Uuid) -> Result<(), OkrRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // 1. Fetch KR to know parent objective
        let kr_opt = sqlx::query_as::<_, KeyResult>("SELECT * FROM key_results WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        let kr = kr_opt.ok_or(OkrRepositoryError::KeyResultNotFound(id))?;

        // 2. Delete the KR
        sqlx::query("DELETE FROM key_results WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        // 3. Recalculate parent objective
        let obj_opt = sqlx::query_as::<_, Objective>("SELECT * FROM objectives WHERE id = ?")
            .bind(kr.objective_id)
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
                WHERE id = ?
                ",
            )
            .bind(obj.progress_percentage)
            .bind(obj.is_completed)
            .bind(obj.updated_at)
            .bind(obj.id)
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

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new(
            "Achieve Peak Fitness",
            Some("Q4 OKR".into()),
            "2026-Q4",
            start,
            end,
        );
        repo.create_objective(&obj).await.unwrap();

        let kr = KeyResult::new_numeric(obj.id, "Run 150 km", 0.0, 150.0, "km", 1.0);
        repo.create_key_result(&kr).await.unwrap();

        let loaded = repo
            .get_objective_with_krs(obj.id)
            .await
            .unwrap()
            .expect("must exist");
        assert_eq!(loaded.objective.id, obj.id);
        assert_eq!(loaded.key_results.len(), 1);
        assert_eq!(loaded.key_results[0].title, "Run 150 km");
    }

    #[tokio::test]
    async fn test_atomic_key_result_update_and_parent_recalculation() {
        let repo = setup_test_repo().await;

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Master Rust", None, "2026-Q4", start, end);
        repo.create_objective(&obj).await.unwrap();

        // KR 1: 100 pages read (weight 1.0)
        let kr1 = KeyResult::new_numeric(obj.id, "Read The Book", 0.0, 100.0, "pages", 1.0);
        repo.create_key_result(&kr1).await.unwrap();

        // KR 2: 10 Projects built (weight 1.0)
        let kr2 = KeyResult::new_numeric(obj.id, "Build Projects", 0.0, 10.0, "projects", 1.0);
        repo.create_key_result(&kr2).await.unwrap();

        // Parent starts at 0%
        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap().unwrap();
        assert_eq!(loaded.objective.progress_percentage, 0.0);

        // Update KR 1 to 50 pages (50% progress for KR 1)
        // Parent progress: (50% * 1.0 + 0% * 1.0) / 2 = 25.0%
        let updated_kr1 = repo.update_key_result_value(kr1.id, 50.0).await.unwrap();
        assert_eq!(updated_kr1.current_value, 50.0);
        assert_eq!(updated_kr1.progress_percentage, 50.0);

        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap().unwrap();
        assert_eq!(loaded.objective.progress_percentage, 25.0);
        assert!(!loaded.objective.is_completed);

        // Update KR 2 to 10 projects (100% progress for KR 2)
        // Parent progress: (50% * 1.0 + 100% * 1.0) / 2 = 75.0%
        repo.update_key_result_value(kr2.id, 10.0).await.unwrap();
        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap().unwrap();
        assert_eq!(loaded.objective.progress_percentage, 75.0);

        // Update KR 1 to 100 pages (100%) -> Parent reaches 100% and is_completed = true!
        repo.update_key_result_value(kr1.id, 100.0).await.unwrap();
        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap().unwrap();
        assert_eq!(loaded.objective.progress_percentage, 100.0);
        assert!(loaded.objective.is_completed);
    }

    #[tokio::test]
    async fn test_list_objectives_by_quarter() {
        let repo = setup_test_repo().await;

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();

        let obj_q4 = Objective::new("Q4 Goal", None, "2026-Q4", start, end);
        let obj_annual = Objective::new("Annual Goal", None, "2026-Annual", start, end);

        repo.create_objective(&obj_q4).await.unwrap();
        repo.create_objective(&obj_annual).await.unwrap();

        let q4_list = repo.list_objectives_by_quarter("2026-Q4").await.unwrap();
        assert_eq!(q4_list.len(), 1);
        assert_eq!(q4_list[0].objective.title, "Q4 Goal");

        let all_list = repo.list_all_objectives().await.unwrap();
        assert_eq!(all_list.len(), 2);
    }

    #[tokio::test]
    async fn test_cascade_delete_objective_and_delete_key_result() {
        let repo = setup_test_repo().await;

        let start = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let obj = Objective::new("Temporary Goal", None, "2026-Q4", start, end);
        repo.create_objective(&obj).await.unwrap();

        let kr1 = KeyResult::new_numeric(obj.id, "KR 1", 0.0, 100.0, "units", 1.0);
        let kr2 = KeyResult::new_numeric(obj.id, "KR 2", 0.0, 100.0, "units", 1.0);
        repo.create_key_result(&kr1).await.unwrap();
        repo.create_key_result(&kr2).await.unwrap();

        // Delete KR 1
        repo.delete_key_result(kr1.id).await.unwrap();
        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap().unwrap();
        assert_eq!(loaded.key_results.len(), 1);
        assert_eq!(loaded.key_results[0].id, kr2.id);

        // Delete Objective (cascades to KR 2)
        repo.delete_objective(obj.id).await.unwrap();
        let loaded = repo.get_objective_with_krs(obj.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_not_found_errors() {
        let repo = setup_test_repo().await;
        let random_id = Uuid::new_v4();

        let res = repo.update_key_result_value(random_id, 10.0).await;
        assert!(matches!(res, Err(OkrRepositoryError::KeyResultNotFound(_))));

        let res = repo.update_objective_progress(random_id, 50.0).await;
        assert!(matches!(res, Err(OkrRepositoryError::ObjectiveNotFound(_))));

        let res = repo.delete_objective(random_id).await;
        assert!(matches!(res, Err(OkrRepositoryError::ObjectiveNotFound(_))));
    }
}
