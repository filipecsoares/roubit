-- 0010_journal_unique_user_date.sql
-- Enforce at most one journal entry per (user_id, entry_date) to prevent TOCTOU duplicates (IMP-01.3)

CREATE UNIQUE INDEX IF NOT EXISTS idx_journal_unique_user_date
    ON journal_entries(user_id, entry_date);
