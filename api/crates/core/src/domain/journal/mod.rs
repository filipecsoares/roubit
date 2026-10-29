//! Encrypted Journaling Subsystem (RF04 - Dual-Storage Architecture).
//!
//! Provides selective field-level encryption for personal reflections, ensuring plaintext
//! content is never stored on disk or in the database unencrypted, while maintaining open
//! queryable metadata (entry dates, mood scores, tags, and timestamps).

pub mod models;

pub use models::{
    deserialize_tags, serialize_tags, DecryptedJournalEntry, EncryptedJournalEntry,
    JournalMetadata, JournalModelError,
};
