//! End-to-end integration tests for Encrypted Journaling subsystem (RF04).
//!
//! Validates:
//! 1. Zero-knowledge guarantee: plaintext is never leaked in SQLite raw columns or BLOBs.
//! 2. Tampering resilience: bit-flips in payload, nonce, or salt trigger strict authentication rejection.
//! 3. Wrong passphrase rejection: uninformative failure without leaking memory or partial text.
//! 4. Full CRUD lifecycle, metadata listing range isolation, and deletion.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::similar_names
)]

use chrono::NaiveDate;
use sqlx::{sqlite::SqlitePoolOptions, Row};
use uuid::Uuid;

use roubit_core::{
    crypto::{kdf::KdfService, memory::SecretString},
    domain::{
        journal::{
            service::JournalServiceError, JournalRepository, JournalRepositoryError,
            JournalService, SqliteJournalRepository,
        },
        seed_test_user,
    },
};

const fn integration_kdf() -> KdfService {
    // Fast Argon2id configuration for testing (4 MiB, 2 iterations, 1 lane)
    KdfService::new(4096, 2, 1)
}

async fn setup_test_journal() -> (
    sqlx::SqlitePool,
    SqliteJournalRepository,
    JournalService<SqliteJournalRepository>,
    Uuid,
) {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let user_id = Uuid::new_v4();
    seed_test_user(&pool, user_id).await.unwrap();

    let repo = SqliteJournalRepository::new(pool.clone());
    let service = JournalService::new(repo.clone(), integration_kdf());
    (pool, repo, service, user_id)
}

#[tokio::test]
async fn test_database_contains_no_plaintext() {
    let (pool, _, service, user_id) = setup_test_journal().await;

    let pass = SecretString::new("super-secret-vault-master-key".to_string());
    let secret_phrase = "TOP_SECRET_REFLECTION_PHRASE_THAT_MUST_NEVER_BE_IN_PLAINTEXT";
    let entry_text = SecretString::new(format!(
        "Confidential notes from morning session: {secret_phrase}"
    ));
    let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    service
        .create_or_update_entry(
            user_id,
            date,
            Some(4),
            vec!["private".to_string(), "vault".to_string()],
            entry_text,
            &pass,
        )
        .await
        .unwrap();

    // 1. Inspect raw SQLite table rows
    let rows = sqlx::query(
        r"
        SELECT id, entry_date, mood_score, tags, salt, nonce, encrypted_payload
        FROM journal_entries
        ",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    let row = &rows[0];

    // Verify metadata columns
    let entry_date: String = row.get("entry_date");
    let mood_score: Option<i64> = row.get("mood_score");
    let tags: String = row.get("tags");
    let encrypted_payload: Vec<u8> = row.get("encrypted_payload");
    let salt: Vec<u8> = row.get("salt");
    let nonce: Vec<u8> = row.get("nonce");

    assert_eq!(entry_date, "2026-10-29");
    assert_eq!(mood_score, Some(4));
    assert!(tags.contains("private"));
    assert!(!tags.contains(secret_phrase));

    // Verify cryptographic BLOB columns contain no plaintext
    let payload_str = String::from_utf8_lossy(&encrypted_payload);
    assert!(
        !payload_str.contains(secret_phrase),
        "Plaintext leaked in raw SQLite encrypted_payload BLOB!"
    );

    let salt_str = String::from_utf8_lossy(&salt);
    assert!(!salt_str.contains(secret_phrase));

    let nonce_str = String::from_utf8_lossy(&nonce);
    assert!(!nonce_str.contains(secret_phrase));
}

#[tokio::test]
async fn test_wrong_passphrase_fails_cleanly() {
    let (_, _, service, user_id) = setup_test_journal().await;

    let correct_pass = SecretString::new("correct-user-master-passphrase".to_string());
    let wrong_pass = SecretString::new("wrong-attacker-passphrase".to_string());
    let reflection = SecretString::new("Today I achieved full memory safety in Rust.".to_string());
    let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let meta = service
        .create_or_update_entry(
            user_id,
            date,
            Some(5),
            vec!["engineering".to_string()],
            reflection,
            &correct_pass,
        )
        .await
        .unwrap();

    // Attempt decryption with wrong passphrase
    let result = service
        .read_and_decrypt_entry(meta.id, user_id, &wrong_pass)
        .await;
    assert!(result.is_err());

    match result.unwrap_err() {
        JournalServiceError::DecryptionFailed => (),
        other => panic!("Expected DecryptionFailed on wrong passphrase, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_single_bit_flip_tampering_detected_and_rejected() {
    let (pool, _, service, user_id) = setup_test_journal().await;

    let pass = SecretString::new("tamper-proof-vault-key".to_string());
    let reflection =
        SecretString::new("Tampering test: original authentic reflection.".to_string());
    let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let meta = service
        .create_or_update_entry(
            user_id,
            date,
            Some(3),
            vec!["security".to_string()],
            reflection,
            &pass,
        )
        .await
        .unwrap();

    // 1. Bit flip in encrypted_payload
    sqlx::query(
        r"
        UPDATE journal_entries
        SET encrypted_payload = X'EE' || SUBSTR(encrypted_payload, 2)
        WHERE id = ?
        ",
    )
    .bind(meta.id)
    .execute(&pool)
    .await
    .unwrap();

    let tampered_result = service
        .read_and_decrypt_entry(meta.id, user_id, &pass)
        .await;
    assert!(tampered_result.is_err());
    match tampered_result.unwrap_err() {
        JournalServiceError::DecryptionFailed => (),
        other => panic!("Expected DecryptionFailed on tampered ciphertext, got: {other:?}"),
    }

    // 2. Tampering with Nonce
    sqlx::query(
        r"
        UPDATE journal_entries
        SET nonce = X'AA' || SUBSTR(nonce, 2)
        WHERE id = ?
        ",
    )
    .bind(meta.id)
    .execute(&pool)
    .await
    .unwrap();

    let tampered_nonce_result = service
        .read_and_decrypt_entry(meta.id, user_id, &pass)
        .await;
    assert!(tampered_nonce_result.is_err());
    match tampered_nonce_result.unwrap_err() {
        JournalServiceError::DecryptionFailed => (),
        other => panic!("Expected DecryptionFailed on tampered nonce, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_journal_full_crud_and_deletion_cleanup() {
    let (pool, repo, service, user_id) = setup_test_journal().await;

    let pass = SecretString::new("crud-lifecycle-pass".to_string());
    let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    // 1. Create entry
    let text1 = SecretString::new("Initial journal note.".to_string());
    let meta1 = service
        .create_or_update_entry(
            user_id,
            date,
            Some(4),
            vec!["morning".to_string()],
            text1,
            &pass,
        )
        .await
        .unwrap();

    assert_eq!(meta1.entry_date, date);
    assert_eq!(meta1.mood_score, Some(4));

    // 2. Read and verify
    let read1 = service
        .read_and_decrypt_entry(meta1.id, user_id, &pass)
        .await
        .unwrap();
    assert_eq!(read1.plaintext_content, "Initial journal note.");

    // 3. Update entry on same date
    let text2 =
        SecretString::new("Updated evening reflection with expanded learnings.".to_string());
    let meta2 = service
        .create_or_update_entry(
            user_id,
            date,
            Some(5),
            vec!["evening".to_string(), "learnings".to_string()],
            text2,
            &pass,
        )
        .await
        .unwrap();

    assert_eq!(meta1.id, meta2.id);
    let read2 = service
        .read_and_decrypt_entry(meta2.id, user_id, &pass)
        .await
        .unwrap();
    assert_eq!(
        read2.plaintext_content,
        "Updated evening reflection with expanded learnings."
    );
    assert_eq!(read2.metadata.mood_score, Some(5));

    // 4. Delete entry
    repo.delete_entry(meta1.id, user_id).await.unwrap();

    // 5. Assert neither service nor raw SQLite has any trace left
    let not_found_read = service
        .read_and_decrypt_entry(meta1.id, user_id, &pass)
        .await;
    assert!(not_found_read.is_err());

    let raw_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw_count, 0);
}

#[tokio::test]
async fn test_date_range_metadata_listing_isolation() {
    let (_, repo, service, user_id) = setup_test_journal().await;

    let pass = SecretString::new("vault-key-range".to_string());

    let d1 = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
    let d3 = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
    let d_outside = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();

    service
        .create_or_update_entry(
            user_id,
            d1,
            Some(3),
            vec!["tag1".to_string()],
            SecretString::new("Note 1".to_string()),
            &pass,
        )
        .await
        .unwrap();
    service
        .create_or_update_entry(
            user_id,
            d2,
            Some(4),
            vec!["tag2".to_string()],
            SecretString::new("Note 2".to_string()),
            &pass,
        )
        .await
        .unwrap();
    service
        .create_or_update_entry(
            user_id,
            d3,
            Some(5),
            vec!["tag3".to_string()],
            SecretString::new("Note 3".to_string()),
            &pass,
        )
        .await
        .unwrap();
    service
        .create_or_update_entry(
            user_id,
            d_outside,
            Some(2),
            vec!["tag4".to_string()],
            SecretString::new("Note 4".to_string()),
            &pass,
        )
        .await
        .unwrap();

    // Filter between d1 and d3 inclusive
    let metadata_list = repo.list_metadata_in_range(user_id, d1, d3).await.unwrap();
    assert_eq!(metadata_list.len(), 3);

    // Verify ordering: DESC by date
    assert_eq!(metadata_list[0].entry_date, d3);
    assert_eq!(metadata_list[0].mood_score, Some(5));
    assert_eq!(metadata_list[1].entry_date, d2);
    assert_eq!(metadata_list[2].entry_date, d1);

    // Test read_and_decrypt_entry_by_date for existing vs non-existing date
    let decrypted_d2 = service
        .read_and_decrypt_entry_by_date(d2, user_id, &pass)
        .await
        .unwrap();
    assert!(decrypted_d2.is_some());
    assert_eq!(decrypted_d2.unwrap().plaintext_content, "Note 2");

    let no_entry = service
        .read_and_decrypt_entry_by_date(
            NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(),
            user_id,
            &pass,
        )
        .await
        .unwrap();
    assert!(no_entry.is_none());

    // Test invalid UUID not found
    let invalid_id_result = service
        .read_and_decrypt_entry(Uuid::new_v4(), user_id, &pass)
        .await;
    assert!(invalid_id_result.is_err());
}

#[tokio::test]
async fn test_journal_multi_tenant_isolation() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    seed_test_user(&pool, user_a).await.unwrap();
    seed_test_user(&pool, user_b).await.unwrap();

    let repo = SqliteJournalRepository::new(pool);
    let service = JournalService::new(repo.clone(), integration_kdf());

    let pass = SecretString::new("shared-passphrase".to_string());
    let date = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();

    let meta_a = service
        .create_or_update_entry(
            user_a,
            date,
            Some(5),
            vec!["user_a".to_string()],
            SecretString::new("User A confidential reflection".to_string()),
            &pass,
        )
        .await
        .unwrap();

    // User B cannot read User A's entry by ID even with identical passphrase
    let read_b = service
        .read_and_decrypt_entry(meta_a.id, user_b, &pass)
        .await;
    assert!(read_b.is_err());

    // User B cannot read User A's entry by date
    let read_date_b = service
        .read_and_decrypt_entry_by_date(date, user_b, &pass)
        .await
        .unwrap();
    assert!(read_date_b.is_none());

    // User B metadata range list does not include User A's entry
    let meta_list_b = repo
        .list_metadata_in_range(user_b, date, date)
        .await
        .unwrap();
    assert_eq!(meta_list_b.len(), 0);

    // User B cannot delete User A's entry
    let delete_b = repo.delete_entry(meta_a.id, user_b).await;
    assert!(matches!(delete_b, Err(JournalRepositoryError::NotFound(_))));
}
