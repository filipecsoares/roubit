//! Asynchronous SQLite repository for Encrypted Journaling (RF04).

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::models::{EncryptedJournalEntry, JournalMetadata, JournalModelError};

/// Errors encountered during Journal repository operations.
#[derive(Debug, thiserror::Error)]
pub enum JournalRepositoryError {
    /// Low-level database error from `SQLx`.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Target journal entry was not found.
    #[error("Journal entry not found: {0}")]
    NotFound(Uuid),

    /// Domain model validation or serialization error.
    #[error("Journal model error: {0}")]
    Model(#[from] JournalModelError),
}

/// Asynchronous repository contract for encrypted journal storage and metadata retrieval.
#[async_trait]
pub trait JournalRepository: Send + Sync {
    /// Persists an encrypted journal entry.
    ///
    /// Inserts a new row or updates an existing row if `entry.metadata.id` already exists.
    ///
    /// # Errors
    /// Returns [`JournalRepositoryError`] on database failure or serialization issues.
    async fn save_entry(&self, entry: &EncryptedJournalEntry)
        -> Result<(), JournalRepositoryError>;

    /// Fetches an encrypted journal entry by its unique identifier.
    ///
    /// # Errors
    /// Returns [`JournalRepositoryError`] on database failure.
    async fn get_entry_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<EncryptedJournalEntry>, JournalRepositoryError>;

    /// Fetches the most recent encrypted journal entry for a given calendar date.
    ///
    /// # Errors
    /// Returns [`JournalRepositoryError`] on database failure.
    async fn get_entry_by_date(
        &self,
        date: NaiveDate,
    ) -> Result<Option<EncryptedJournalEntry>, JournalRepositoryError>;

    /// Lists open metadata for all journal entries within the inclusive date range `[start_date, end_date]`.
    ///
    /// Omits heavy ciphertext payload BLOBs from the SQL projection, computing only `LENGTH(encrypted_payload)`
    /// for fast overview lists and minimal memory consumption.
    ///
    /// # Errors
    /// Returns [`JournalRepositoryError`] on database failure.
    async fn list_metadata_in_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, JournalRepositoryError>;

    /// Deletes a journal entry by its unique identifier.
    ///
    /// # Errors
    /// Returns [`JournalRepositoryError::NotFound`] if no row was deleted, or [`JournalRepositoryError::Database`]
    /// on database failure.
    async fn delete_entry(&self, id: Uuid) -> Result<(), JournalRepositoryError>;
}

/// SQLite implementation of [`JournalRepository`].
#[derive(Debug, Clone)]
pub struct SqliteJournalRepository {
    pool: SqlitePool,
}

impl SqliteJournalRepository {
    /// Creates a new repository instance backed by the provided `SqlitePool`.
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the underlying `SqlitePool`.
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl JournalRepository for SqliteJournalRepository {
    async fn save_entry(
        &self,
        entry: &EncryptedJournalEntry,
    ) -> Result<(), JournalRepositoryError> {
        let tags_json = entry.metadata.tags_to_json()?;

        sqlx::query(
            r"
            INSERT INTO journal_entries (
                id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                entry_date = excluded.entry_date,
                mood_score = excluded.mood_score,
                tags = excluded.tags,
                salt = excluded.salt,
                nonce = excluded.nonce,
                encrypted_payload = excluded.encrypted_payload,
                updated_at = excluded.updated_at
            ",
        )
        .bind(entry.metadata.id)
        .bind(entry.metadata.entry_date)
        .bind(entry.metadata.mood_score.map(i64::from))
        .bind(&tags_json)
        .bind(&entry.salt[..])
        .bind(&entry.nonce[..])
        .bind(&entry.encrypted_payload[..])
        .bind(entry.metadata.created_at)
        .bind(entry.metadata.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_entry_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<EncryptedJournalEntry>, JournalRepositoryError> {
        let entry = sqlx::query_as::<_, EncryptedJournalEntry>(
            "SELECT * FROM journal_entries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    async fn get_entry_by_date(
        &self,
        date: NaiveDate,
    ) -> Result<Option<EncryptedJournalEntry>, JournalRepositoryError> {
        let entry = sqlx::query_as::<_, EncryptedJournalEntry>(
            "SELECT * FROM journal_entries WHERE entry_date = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    async fn list_metadata_in_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<JournalMetadata>, JournalRepositoryError> {
        let metadata = sqlx::query_as::<_, JournalMetadata>(
            r"
            SELECT id, entry_date, mood_score, tags, LENGTH(encrypted_payload) as ciphertext_bytes_len, created_at, updated_at
            FROM journal_entries
            WHERE entry_date BETWEEN ? AND ?
            ORDER BY entry_date DESC, created_at DESC
            ",
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(metadata)
    }

    async fn delete_entry(&self, id: Uuid) -> Result<(), JournalRepositoryError> {
        let result = sqlx::query("DELETE FROM journal_entries WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(JournalRepositoryError::NotFound(id));
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::sqlite::SqlitePoolOptions;

    fn mock_entry(date: NaiveDate, mood: Option<u8>, tags: Vec<String>) -> EncryptedJournalEntry {
        let meta = JournalMetadata::new(date, mood, tags, 32).unwrap();
        let salt = [42u8; 16];
        let nonce = [7u8; 12];
        let payload = vec![
            0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
        ];
        EncryptedJournalEntry::new(meta, salt, nonce, payload).unwrap()
    }

    #[tokio::test]
    async fn test_journal_repository_crud_lifecycle() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool);

        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let entry = mock_entry(date, Some(5), vec!["productivity".to_string()]);
        let entry_id = entry.metadata.id;

        // 1. Save entry
        repo.save_entry(&entry).await.unwrap();

        // 2. Fetch by ID
        let fetched = repo.get_entry_by_id(entry_id).await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.metadata.id, entry_id);
        assert_eq!(fetched.metadata.entry_date, date);
        assert_eq!(fetched.metadata.mood_score, Some(5));
        assert_eq!(fetched.salt, entry.salt);
        assert_eq!(fetched.nonce, entry.nonce);
        assert_eq!(fetched.encrypted_payload, entry.encrypted_payload);

        // 3. Fetch by Date
        let fetched_by_date = repo.get_entry_by_date(date).await.unwrap();
        assert!(fetched_by_date.is_some());
        assert_eq!(fetched_by_date.unwrap().metadata.id, entry_id);

        // 4. Update entry via UPSERT
        let mut updated_entry = entry.clone();
        updated_entry.metadata.mood_score = Some(4);
        updated_entry.metadata.tags = vec!["reflection".to_string(), "evening".to_string()];
        updated_entry.metadata.updated_at = Utc::now();
        updated_entry.encrypted_payload = vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6];

        repo.save_entry(&updated_entry).await.unwrap();

        let refetched = repo.get_entry_by_id(entry_id).await.unwrap().unwrap();
        assert_eq!(refetched.metadata.mood_score, Some(4));
        assert_eq!(
            refetched.metadata.tags,
            vec!["reflection".to_string(), "evening".to_string()]
        );
        assert_eq!(refetched.encrypted_payload, updated_entry.encrypted_payload);

        // 5. Delete entry
        repo.delete_entry(entry_id).await.unwrap();
        let after_delete = repo.get_entry_by_id(entry_id).await.unwrap();
        assert!(after_delete.is_none());

        // 6. Delete again returns NotFound
        let delete_err = repo.delete_entry(entry_id).await.unwrap_err();
        match delete_err {
            JournalRepositoryError::NotFound(id) => assert_eq!(id, entry_id),
            other => panic!("Expected NotFound error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_journal_repository_metadata_listing_in_range() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool);

        let d1 = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let d_outside = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();

        let e1 = mock_entry(d1, Some(3), vec!["chill".to_string()]);
        let e2 = mock_entry(d2, Some(4), vec!["focus".to_string()]);
        let e3 = mock_entry(d3, Some(5), vec!["super".to_string()]);
        let e_out = mock_entry(d_outside, Some(2), vec!["late".to_string()]);

        repo.save_entry(&e1).await.unwrap();
        repo.save_entry(&e2).await.unwrap();
        repo.save_entry(&e3).await.unwrap();
        repo.save_entry(&e_out).await.unwrap();

        let metadata_list = repo.list_metadata_in_range(d1, d3).await.unwrap();

        assert_eq!(metadata_list.len(), 3);
        // Ordered by entry_date DESC
        assert_eq!(metadata_list[0].entry_date, d3);
        assert_eq!(metadata_list[0].mood_score, Some(5));
        assert_eq!(metadata_list[0].tags, vec!["super".to_string()]);
        assert_eq!(metadata_list[0].ciphertext_bytes_len, 16);

        assert_eq!(metadata_list[1].entry_date, d2);
        assert_eq!(metadata_list[2].entry_date, d1);
    }
}
