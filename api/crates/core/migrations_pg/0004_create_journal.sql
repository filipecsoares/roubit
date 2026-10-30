-- 0004_create_journal.sql
-- Encrypted Journaling Schema (RF04 - Dual-Storage Architecture)

CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    entry_date DATE NOT NULL,
    mood_score INTEGER CHECK(mood_score IS NULL OR (mood_score >= 1 AND mood_score <= 5)),
    tags TEXT NOT NULL DEFAULT '[]',
    salt BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    encrypted_payload BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pg_journal_entries_date ON journal_entries(entry_date DESC);
CREATE INDEX IF NOT EXISTS idx_pg_journal_entries_mood ON journal_entries(mood_score);
