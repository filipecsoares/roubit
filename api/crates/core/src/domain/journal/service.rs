//! Cryptographic Application Service for Encrypted Journaling (RF04).
//!
//! Coordinates Argon2id key derivation, AES-256-GCM authenticated encryption,
//! and persistent dual-storage in the [`JournalRepository`].

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use crate::crypto::{
    cipher::{AesGcmCipher, CipherError, EncryptedEnvelope},
    kdf::{KdfError, KdfService},
    memory::SecretString,
};

use super::{
    models::{DecryptedJournalEntry, EncryptedJournalEntry, JournalMetadata, JournalModelError},
    repository::{JournalRepository, JournalRepositoryError},
};

/// Errors originating from journal encryption, decryption, or storage workflows.
#[derive(Debug, thiserror::Error)]
pub enum JournalServiceError {
    /// Failure in underlying journal repository.
    #[error("Repository error: {0}")]
    Repository(#[from] JournalRepositoryError),

    /// Key derivation failure from Argon2id KDF.
    #[error("Crypto KDF error: {0}")]
    Kdf(#[from] KdfError),

    /// Symmetric cipher encryption or authentication failure.
    #[error("Cipher error: {0}")]
    Cipher(#[from] CipherError),

    /// Decryption failed due to an invalid passphrase or corrupted/tampered data.
    #[error("Decryption failed: wrong passphrase or corrupted data")]
    DecryptionFailed,

    /// Domain model validation error.
    #[error("Journal model error: {0}")]
    Model(#[from] JournalModelError),

    /// Plaintext UTF-8 decode failure.
    #[error("UTF-8 decoding error for decrypted plaintext: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Unified application service for zero-knowledge encrypted journaling.
#[derive(Debug)]
pub struct JournalService<R: JournalRepository> {
    repo: R,
    kdf: KdfService,
}

impl<R: JournalRepository> JournalService<R> {
    /// Creates a new `JournalService` with the specified repository and key derivation service.
    #[must_use]
    pub const fn new(repo: R, kdf: KdfService) -> Self {
        Self { repo, kdf }
    }

    /// Accessor for the inner repository reference.
    #[must_use]
    pub const fn repository(&self) -> &R {
        &self.repo
    }

    /// Accessor for the inner KDF service reference.
    #[must_use]
    pub const fn kdf(&self) -> &KdfService {
        &self.kdf
    }

    /// Encrypts and persists a journal entry for the specified calendar date.
    ///
    /// If an entry already exists for `date`, its identifier and creation timestamp are preserved,
    /// updating the entry content, salt, nonce, and ciphertext payload.
    ///
    /// Key derivation material (`DerivedKey`) and `plaintext` are dropped and zeroized from RAM
    /// upon function exit.
    ///
    /// # Errors
    /// Returns [`JournalServiceError`] if KDF, cipher, or repository fails.
    pub async fn create_or_update_entry(
        &self,
        date: NaiveDate,
        mood: Option<u8>,
        tags: Vec<String>,
        plaintext: SecretString,
        passphrase: &SecretString,
    ) -> Result<JournalMetadata, JournalServiceError> {
        JournalMetadata::validate_mood(mood)?;

        // 1. Generate fresh 16-byte salt for this entry
        let salt = KdfService::generate_salt();

        // 2. Derive 256-bit symmetric encryption key from passphrase + salt
        let derived_key = self.kdf.derive_key(passphrase.as_bytes(), &salt)?;

        // 3. Encrypt plaintext reflection via AES-256-GCM
        let envelope = AesGcmCipher::encrypt(derived_key.as_bytes(), plaintext.as_bytes(), None)?;

        // 4. Resolve entry ID and created_at if updating an existing daily record
        let existing = self.repo.get_entry_by_date(date).await?;
        let (id, created_at) = if let Some(old) = existing {
            (old.metadata.id, old.metadata.created_at)
        } else {
            (Uuid::new_v4(), Utc::now())
        };

        let now = Utc::now();
        let metadata = JournalMetadata {
            id,
            entry_date: date,
            mood_score: mood,
            tags,
            ciphertext_bytes_len: envelope.ciphertext.len(),
            created_at,
            updated_at: now,
        };

        // 5. Build encrypted envelope entity and save to repository
        let encrypted_entry = EncryptedJournalEntry::new(
            metadata.clone(),
            salt,
            envelope.nonce,
            envelope.ciphertext,
        )?;

        self.repo.save_entry(&encrypted_entry).await?;

        Ok(metadata)
    }

    /// Reads, authenticates, and decrypts a journal entry by its unique identifier.
    ///
    /// # Errors
    /// - Returns [`JournalRepositoryError::NotFound`] if no entry with `id` exists.
    /// - Returns [`JournalServiceError::DecryptionFailed`] if the passphrase is incorrect or the ciphertext was altered.
    pub async fn read_and_decrypt_entry(
        &self,
        id: Uuid,
        passphrase: &SecretString,
    ) -> Result<DecryptedJournalEntry, JournalServiceError> {
        let entry = self
            .repo
            .get_entry_by_id(id)
            .await?
            .ok_or(JournalRepositoryError::NotFound(id))?;

        let derived_key = self.kdf.derive_key(passphrase.as_bytes(), &entry.salt)?;

        let envelope = EncryptedEnvelope {
            nonce: entry.nonce,
            ciphertext: entry.encrypted_payload,
        };

        let plaintext_bytes = AesGcmCipher::decrypt(derived_key.as_bytes(), &envelope, None)
            .map_err(|_| JournalServiceError::DecryptionFailed)?;

        let plaintext_content = String::from_utf8(plaintext_bytes)?;

        Ok(DecryptedJournalEntry::new(
            entry.metadata,
            plaintext_content,
        ))
    }

    /// Reads, authenticates, and decrypts the latest journal entry for a given calendar date.
    ///
    /// Returns `Ok(None)` if no entry exists for `date`.
    ///
    /// # Errors
    /// Returns [`JournalServiceError::DecryptionFailed`] on invalid passphrase or tampering.
    pub async fn read_and_decrypt_entry_by_date(
        &self,
        date: NaiveDate,
        passphrase: &SecretString,
    ) -> Result<Option<DecryptedJournalEntry>, JournalServiceError> {
        let entry_opt = self.repo.get_entry_by_date(date).await?;

        let Some(entry) = entry_opt else {
            return Ok(None);
        };

        let derived_key = self.kdf.derive_key(passphrase.as_bytes(), &entry.salt)?;

        let envelope = EncryptedEnvelope {
            nonce: entry.nonce,
            ciphertext: entry.encrypted_payload,
        };

        let plaintext_bytes = AesGcmCipher::decrypt(derived_key.as_bytes(), &envelope, None)
            .map_err(|_| JournalServiceError::DecryptionFailed)?;

        let plaintext_content = String::from_utf8(plaintext_bytes)?;

        Ok(Some(DecryptedJournalEntry::new(
            entry.metadata,
            plaintext_content,
        )))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::journal::repository::SqliteJournalRepository;
    use sqlx::sqlite::SqlitePoolOptions;

    fn test_kdf() -> KdfService {
        // Fast KDF for unit testing to avoid long Argon2 iteration delays
        KdfService::new(1024, 1, 1)
    }

    #[tokio::test]
    async fn test_journal_service_encryption_and_decryption_roundtrip() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool);
        let service = JournalService::new(repo, test_kdf());

        let pass = SecretString::new("my-master-vault-password".to_string());
        let reflection = SecretString::new(
            "Today was highly productive with deep focus on Rust architecture.".to_string(),
        );
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        let meta = service
            .create_or_update_entry(
                date,
                Some(5),
                vec!["focus".to_string(), "rust".to_string()],
                reflection,
                &pass,
            )
            .await
            .unwrap();

        assert_eq!(meta.entry_date, date);
        assert_eq!(meta.mood_score, Some(5));

        // Read and decrypt by ID
        let decrypted = service
            .read_and_decrypt_entry(meta.id, &pass)
            .await
            .unwrap();
        assert_eq!(
            decrypted.plaintext_content,
            "Today was highly productive with deep focus on Rust architecture."
        );
        assert_eq!(decrypted.metadata.id, meta.id);

        // Read and decrypt by Date
        let decrypted_by_date = service
            .read_and_decrypt_entry_by_date(date, &pass)
            .await
            .unwrap();
        assert!(decrypted_by_date.is_some());
        assert_eq!(
            decrypted_by_date.unwrap().plaintext_content,
            "Today was highly productive with deep focus on Rust architecture."
        );
    }

    #[tokio::test]
    async fn test_journal_service_wrong_passphrase_fails_cleanly() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool);
        let service = JournalService::new(repo, test_kdf());

        let correct_pass = SecretString::new("correct-password".to_string());
        let wrong_pass = SecretString::new("wrong-password".to_string());
        let reflection = SecretString::new("Confidential personal thoughts.".to_string());
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        let meta = service
            .create_or_update_entry(date, Some(4), vec![], reflection, &correct_pass)
            .await
            .unwrap();

        let err = service
            .read_and_decrypt_entry(meta.id, &wrong_pass)
            .await
            .unwrap_err();

        match err {
            JournalServiceError::DecryptionFailed => (),
            other => panic!("Expected DecryptionFailed error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_journal_service_tampered_ciphertext_fails_cleanly() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool.clone());
        let service = JournalService::new(repo, test_kdf());

        let pass = SecretString::new("secure-pass".to_string());
        let reflection = SecretString::new("Unbroken integrity test.".to_string());
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        let meta = service
            .create_or_update_entry(date, Some(3), vec![], reflection, &pass)
            .await
            .unwrap();

        // Deliberately tamper with 1 byte of encrypted_payload in SQLite
        sqlx::query(
            r"
            UPDATE journal_entries
            SET encrypted_payload = X'FF' || SUBSTR(encrypted_payload, 2)
            WHERE id = ?
            ",
        )
        .bind(meta.id)
        .execute(&pool)
        .await
        .unwrap();

        let err = service
            .read_and_decrypt_entry(meta.id, &pass)
            .await
            .unwrap_err();

        match err {
            JournalServiceError::DecryptionFailed => (),
            other => panic!("Expected DecryptionFailed error on tampered payload, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_journal_service_update_existing_date_preserves_id() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let repo = SqliteJournalRepository::new(pool);
        let service = JournalService::new(repo, test_kdf());

        let pass = SecretString::new("vault-pass".to_string());
        let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

        // 1. Initial morning entry
        let morning_text = SecretString::new("Morning reflection: energized.".to_string());
        let meta1 = service
            .create_or_update_entry(
                date,
                Some(5),
                vec!["morning".to_string()],
                morning_text,
                &pass,
            )
            .await
            .unwrap();

        // 2. Evening update for same date
        let evening_text = SecretString::new("Evening reflection: day complete.".to_string());
        let meta2 = service
            .create_or_update_entry(
                date,
                Some(4),
                vec!["evening".to_string(), "wrapup".to_string()],
                evening_text,
                &pass,
            )
            .await
            .unwrap();

        // Preserves ID and created_at
        assert_eq!(meta1.id, meta2.id);
        assert_eq!(meta1.created_at, meta2.created_at);
        assert_eq!(meta2.mood_score, Some(4));
        assert_eq!(
            meta2.tags,
            vec!["evening".to_string(), "wrapup".to_string()]
        );

        let decrypted = service
            .read_and_decrypt_entry(meta1.id, &pass)
            .await
            .unwrap();
        assert_eq!(
            decrypted.plaintext_content,
            "Evening reflection: day complete."
        );
    }
}
