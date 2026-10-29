-- 0004_create_journal.sql
-- Encrypted Journaling Schema (RF04 - Dual-Storage Architecture)

CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY NOT NULL,
    entry_date TEXT NOT NULL, -- YYYY-MM-DD
    mood_score INTEGER CHECK(mood_score IS NULL OR (mood_score >= 1 AND mood_score <= 5)),
    tags TEXT NOT NULL DEFAULT '[]', -- JSON array of tags
    salt BLOB NOT NULL,             -- 16 bytes KDF salt
    nonce BLOB NOT NULL,            -- 12 bytes AES-GCM nonce
    encrypted_payload BLOB NOT NULL,-- Ciphertext + Auth Tag
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_journal_entries_date ON journal_entries(entry_date DESC);
CREATE INDEX IF NOT EXISTS idx_journal_entries_mood ON journal_entries(mood_score);
