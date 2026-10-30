//! Domain models for Encrypted Journaling (RF04 - Dual-Storage Architecture).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, FromRow, Row};
use uuid::Uuid;

/// Validation and model errors for journaling entities.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JournalModelError {
    /// Mood score is outside the allowed range of 1 to 5.
    #[error("Mood score must be between 1 and 5, got {0}")]
    InvalidMoodScore(u8),

    /// Salt byte slice length is not 16 bytes.
    #[error("Salt must be exactly 16 bytes, got {got}")]
    InvalidSaltLength {
        /// Actual byte count encountered.
        got: usize,
    },

    /// Nonce byte slice length is not 12 bytes.
    #[error("Nonce must be exactly 12 bytes, got {got}")]
    InvalidNonceLength {
        /// Actual byte count encountered.
        got: usize,
    },

    /// Encrypted payload is empty.
    #[error("Encrypted payload cannot be empty")]
    EmptyPayload,

    /// Tags failed JSON parsing.
    #[error("Failed to parse tags JSON: {0}")]
    InvalidTagsJson(String),
}

/// Open/public metadata of a journal entry, stored unencrypted for fast querying and filtering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalMetadata {
    /// Unique identifier for the journal entry.
    pub id: Uuid,

    /// Calendar date associated with this journal entry.
    pub entry_date: NaiveDate,

    /// Self-reported mood index: 1 (very bad) to 5 (excellent), or `None`.
    pub mood_score: Option<u8>,

    /// Categorical tags/topics assigned to this entry (e.g., `["gratitude", "reflection"]`).
    pub tags: Vec<String>,

    /// Byte size of the stored ciphertext payload (including 16-byte authentication tag).
    pub ciphertext_bytes_len: usize,

    /// UTC timestamp of entry creation.
    pub created_at: DateTime<Utc>,

    /// UTC timestamp of the last update.
    pub updated_at: DateTime<Utc>,
}

impl JournalMetadata {
    /// Creates a new `JournalMetadata` with generated UUID and current UTC timestamps.
    ///
    /// # Errors
    /// Returns [`JournalModelError::InvalidMoodScore`] if `mood_score` is provided and outside `1..=5`.
    pub fn new(
        entry_date: NaiveDate,
        mood_score: Option<u8>,
        tags: Vec<String>,
        ciphertext_bytes_len: usize,
    ) -> Result<Self, JournalModelError> {
        Self::validate_mood(mood_score)?;
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            entry_date,
            mood_score,
            tags,
            ciphertext_bytes_len,
            created_at: now,
            updated_at: now,
        })
    }

    /// Validates that a mood score falls within the allowed 1..=5 range.
    ///
    /// # Errors
    /// Returns [`JournalModelError::InvalidMoodScore`] if mood is outside `1..=5`.
    pub fn validate_mood(mood_score: Option<u8>) -> Result<(), JournalModelError> {
        if let Some(mood) = mood_score {
            if !(1..=5).contains(&mood) {
                return Err(JournalModelError::InvalidMoodScore(mood));
            }
        }
        Ok(())
    }

    /// Serializes the tags vector to a canonical JSON string.
    ///
    /// # Errors
    /// Returns [`JournalModelError::InvalidTagsJson`] if serialization fails.
    pub fn tags_to_json(&self) -> Result<String, JournalModelError> {
        serialize_tags(&self.tags)
    }
}

/// Encrypted storage envelope holding sensitive reflection content, salt, and AES-GCM nonce.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedJournalEntry {
    /// Open metadata describing this entry.
    pub metadata: JournalMetadata,

    /// 16-byte cryptographic salt used for Argon2id key derivation.
    pub salt: [u8; 16],

    /// 12-byte initialization vector / nonce for AES-256-GCM.
    pub nonce: [u8; 12],

    /// Ciphertext payload with appended 16-byte Poly1305 authentication tag.
    pub encrypted_payload: Vec<u8>,
}

impl EncryptedJournalEntry {
    /// Creates a new `EncryptedJournalEntry`.
    ///
    /// # Errors
    /// Returns [`JournalModelError::EmptyPayload`] if `encrypted_payload` is empty.
    pub fn new(
        metadata: JournalMetadata,
        salt: [u8; 16],
        nonce: [u8; 12],
        encrypted_payload: Vec<u8>,
    ) -> Result<Self, JournalModelError> {
        if encrypted_payload.is_empty() {
            return Err(JournalModelError::EmptyPayload);
        }
        Ok(Self {
            metadata,
            salt,
            nonce,
            encrypted_payload,
        })
    }
}

/// Decrypted in-memory representation of a journal entry.
///
/// Plaintext content is held only in RAM and must never be committed to disk or SQLite in plaintext.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecryptedJournalEntry {
    /// Associated open metadata.
    pub metadata: JournalMetadata,

    /// Plaintext reflection content.
    pub plaintext_content: String,
}

impl DecryptedJournalEntry {
    /// Creates a new decrypted entry from metadata and plaintext content.
    #[must_use]
    pub fn new(metadata: JournalMetadata, plaintext_content: impl Into<String>) -> Self {
        Self {
            metadata,
            plaintext_content: plaintext_content.into(),
        }
    }
}

/// Serializes tags into a JSON string array.
///
/// # Errors
/// Returns [`JournalModelError::InvalidTagsJson`] if serialization fails.
pub fn serialize_tags(tags: &[String]) -> Result<String, JournalModelError> {
    serde_json::to_string(tags).map_err(|e| JournalModelError::InvalidTagsJson(e.to_string()))
}

/// Deserializes tags from a JSON string array (defaults to empty on empty input).
///
/// # Errors
/// Returns [`JournalModelError::InvalidTagsJson`] if deserialization fails.
pub fn deserialize_tags(tags_json: &str) -> Result<Vec<String>, JournalModelError> {
    let trimmed = tags_json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(trimmed).map_err(|e| JournalModelError::InvalidTagsJson(e.to_string()))
}

impl<'r> FromRow<'r, SqliteRow> for JournalMetadata {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let entry_date: NaiveDate = row.try_get("entry_date")?;
        let mood_score: Option<u8> = row
            .try_get::<Option<i64>, _>("mood_score")?
            .map(|m| u8::try_from(m).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()?;

        let tags_str: String = row.try_get("tags")?;
        let tags = deserialize_tags(&tags_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::option_if_let_else
        )]
        let ciphertext_bytes_len = if let Ok(len) = row.try_get::<i64, _>("ciphertext_bytes_len") {
            usize::try_from(len).unwrap_or(0)
        } else if let Ok(payload) = row.try_get::<Vec<u8>, _>("encrypted_payload") {
            payload.len()
        } else {
            0
        };

        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        Ok(Self {
            id,
            entry_date,
            mood_score,
            tags,
            ciphertext_bytes_len,
            created_at,
            updated_at,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for EncryptedJournalEntry {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let id: Uuid = row.try_get("id")?;
        let entry_date: NaiveDate = row.try_get("entry_date")?;
        let mood_score: Option<u8> = row
            .try_get::<Option<i64>, _>("mood_score")?
            .map(|m| u8::try_from(m).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()?;

        let tags_str: String = row.try_get("tags")?;
        let tags = deserialize_tags(&tags_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let salt_raw: Vec<u8> = row.try_get("salt")?;
        let nonce_raw: Vec<u8> = row.try_get("nonce")?;
        let encrypted_payload: Vec<u8> = row.try_get("encrypted_payload")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

        let salt: [u8; 16] = salt_raw.try_into().map_err(|vec: Vec<u8>| {
            sqlx::Error::Decode(
                format!("Invalid salt length: expected 16 bytes, got {}", vec.len()).into(),
            )
        })?;
        let nonce: [u8; 12] = nonce_raw.try_into().map_err(|vec: Vec<u8>| {
            sqlx::Error::Decode(
                format!("Invalid nonce length: expected 12 bytes, got {}", vec.len()).into(),
            )
        })?;

        let ciphertext_bytes_len = encrypted_payload.len();

        let metadata = JournalMetadata {
            id,
            entry_date,
            mood_score,
            tags,
            ciphertext_bytes_len,
            created_at,
            updated_at,
        };

        Ok(Self {
            metadata,
            salt,
            nonce,
            encrypted_payload,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_journal_metadata_creation_and_mood_validation() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let tags = vec!["focus".to_string(), "rust".to_string()];

        let meta = JournalMetadata::new(date, Some(5), tags.clone(), 64).unwrap();
        assert_eq!(meta.entry_date, date);
        assert_eq!(meta.mood_score, Some(5));
        assert_eq!(meta.tags, tags);
        assert_eq!(meta.ciphertext_bytes_len, 64);

        // Valid mood edge cases
        assert!(JournalMetadata::new(date, Some(1), vec![], 0).is_ok());
        assert!(JournalMetadata::new(date, None, vec![], 0).is_ok());

        // Invalid mood scores
        let err_zero = JournalMetadata::new(date, Some(0), vec![], 0).unwrap_err();
        assert_eq!(err_zero, JournalModelError::InvalidMoodScore(0));

        let err_six = JournalMetadata::new(date, Some(6), vec![], 0).unwrap_err();
        assert_eq!(err_six, JournalModelError::InvalidMoodScore(6));
    }

    #[test]
    fn test_tags_serialization_and_deserialization() {
        let tags = vec!["deep_work".to_string(), "meditation".to_string()];
        let json = serialize_tags(&tags).unwrap();
        assert_eq!(json, r#"["deep_work","meditation"]"#);

        let parsed = deserialize_tags(&json).unwrap();
        assert_eq!(parsed, tags);

        // Empty tags
        assert_eq!(deserialize_tags("").unwrap(), Vec::<String>::new());
        assert_eq!(deserialize_tags("   ").unwrap(), Vec::<String>::new());
        assert_eq!(deserialize_tags("[]").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_encrypted_entry_empty_payload_rejected() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let meta = JournalMetadata::new(date, Some(4), vec![], 0).unwrap();

        let err = EncryptedJournalEntry::new(meta, [0u8; 16], [0u8; 12], vec![]).unwrap_err();
        assert_eq!(err, JournalModelError::EmptyPayload);
    }

    #[test]
    fn test_decrypted_entry_instantiation() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let meta = JournalMetadata::new(date, Some(4), vec!["gratitude".to_string()], 32).unwrap();
        let decrypted = DecryptedJournalEntry::new(meta.clone(), "Today was an amazing day.");

        assert_eq!(decrypted.metadata, meta);
        assert_eq!(decrypted.plaintext_content, "Today was an amazing day.");
    }

    #[tokio::test]
    async fn test_journal_sqlite_migrations_and_row_mapping() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // 1. Run migrations
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // 2. Insert mock encrypted entry
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let meta =
            JournalMetadata::new(date, Some(5), vec!["productivity".to_string()], 48).unwrap();
        let salt = [7u8; 16];
        let nonce = [9u8; 12];
        let payload = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let entry = EncryptedJournalEntry::new(meta.clone(), salt, nonce, payload.clone()).unwrap();

        let tags_json = entry.metadata.tags_to_json().unwrap();

        sqlx::query(
            r"
            INSERT INTO journal_entries (id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&pool)
        .await
        .unwrap();

        // 3. Query EncryptedJournalEntry using FromRow
        let fetched_entry = sqlx::query_as::<_, EncryptedJournalEntry>(
            "SELECT * FROM journal_entries WHERE id = ?",
        )
        .bind(entry.metadata.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(fetched_entry.metadata.id, entry.metadata.id);
        assert_eq!(fetched_entry.metadata.entry_date, date);
        assert_eq!(fetched_entry.metadata.mood_score, Some(5));
        assert_eq!(
            fetched_entry.metadata.tags,
            vec!["productivity".to_string()]
        );
        assert_eq!(fetched_entry.metadata.ciphertext_bytes_len, payload.len());
        assert_eq!(fetched_entry.salt, salt);
        assert_eq!(fetched_entry.nonce, nonce);
        assert_eq!(fetched_entry.encrypted_payload, payload);

        // 4. Query JournalMetadata with LENGTH(encrypted_payload) as ciphertext_bytes_len
        let fetched_meta = sqlx::query_as::<_, JournalMetadata>(
            r"
            SELECT id, entry_date, mood_score, tags, LENGTH(encrypted_payload) as ciphertext_bytes_len, created_at, updated_at
            FROM journal_entries
            WHERE entry_date = ?
            ",
        )
        .bind(date)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(fetched_meta.id, entry.metadata.id);
        assert_eq!(fetched_meta.mood_score, Some(5));
        assert_eq!(fetched_meta.ciphertext_bytes_len, payload.len());
        assert_eq!(fetched_meta.tags, vec!["productivity".to_string()]);

        // 5. Test mood constraint in SQLite
        let invalid_mood_result = sqlx::query(
            r"
            INSERT INTO journal_entries (id, entry_date, mood_score, tags, salt, nonce, encrypted_payload, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(Uuid::new_v4())
        .bind(date)
        .bind(7i64) // Invalid mood (> 5)
        .bind("[]")
        .bind(&salt[..])
        .bind(&nonce[..])
        .bind(&payload[..])
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await;

        assert!(invalid_mood_result.is_err());
    }
}
