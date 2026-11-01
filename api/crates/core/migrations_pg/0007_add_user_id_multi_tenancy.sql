-- 0007_add_user_id_multi_tenancy.sql
-- Multi-Tenancy Data Isolation Schema (PostgreSQL)

-- 1. Habits: Associate habit definition with an authenticated user
ALTER TABLE habits ADD COLUMN IF NOT EXISTS user_id TEXT REFERENCES users(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_pg_habits_user_archived ON habits(user_id, is_archived);

-- 2. Routines: Associate routine definition with an authenticated user
ALTER TABLE routines ADD COLUMN IF NOT EXISTS user_id TEXT REFERENCES users(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_pg_routines_user_active ON routines(user_id, is_active);

-- 3. Objectives: Associate quarterly OKR with an authenticated user
ALTER TABLE objectives ADD COLUMN IF NOT EXISTS user_id TEXT REFERENCES users(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_pg_objectives_user_quarter ON objectives(user_id, quarter);

-- 4. Journal Entries: Associate encrypted journal reflections with an authenticated user
ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS user_id TEXT REFERENCES users(id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_pg_journal_user_date ON journal_entries(user_id, entry_date DESC);
