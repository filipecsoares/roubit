-- 0007_add_user_id_multi_tenancy.sql
-- Multi-Tenancy Data Isolation Schema (PostgreSQL)

-- 0. Seed default local user for local CLI operations and legacy backfill
INSERT INTO users (id, email, password_hash, created_at)
VALUES ('00000000-0000-0000-0000-000000000001', 'local@roubit.internal', 'disabled', '2026-01-01T00:00:00Z')
ON CONFLICT (id) DO NOTHING;

-- 1. Habits: Associate habit definition with an authenticated user
ALTER TABLE habits ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE habits ALTER COLUMN user_id SET DEFAULT '00000000-0000-0000-0000-000000000001';
UPDATE habits SET user_id = '00000000-0000-0000-0000-000000000001' WHERE user_id IS NULL OR user_id = 'default';
CREATE INDEX IF NOT EXISTS idx_pg_habits_user_archived ON habits(user_id, is_archived);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_habits_user_id') THEN
        ALTER TABLE habits
            ADD CONSTRAINT fk_habits_user_id
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;
END $$;

-- 2. Routines: Associate routine definition with an authenticated user
ALTER TABLE routines ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE routines ALTER COLUMN user_id SET DEFAULT '00000000-0000-0000-0000-000000000001';
UPDATE routines SET user_id = '00000000-0000-0000-0000-000000000001' WHERE user_id IS NULL OR user_id = 'default';
CREATE INDEX IF NOT EXISTS idx_pg_routines_user_active ON routines(user_id, is_active);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_routines_user_id') THEN
        ALTER TABLE routines
            ADD CONSTRAINT fk_routines_user_id
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;
END $$;

-- 3. Objectives: Associate quarterly OKR with an authenticated user
ALTER TABLE objectives ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE objectives ALTER COLUMN user_id SET DEFAULT '00000000-0000-0000-0000-000000000001';
UPDATE objectives SET user_id = '00000000-0000-0000-0000-000000000001' WHERE user_id IS NULL OR user_id = 'default';
CREATE INDEX IF NOT EXISTS idx_pg_objectives_user_quarter ON objectives(user_id, quarter);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_objectives_user_id') THEN
        ALTER TABLE objectives
            ADD CONSTRAINT fk_objectives_user_id
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;
END $$;

-- 4. Journal Entries: Associate encrypted journal reflections with an authenticated user
ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS user_id TEXT;
ALTER TABLE journal_entries ALTER COLUMN user_id SET DEFAULT '00000000-0000-0000-0000-000000000001';
UPDATE journal_entries SET user_id = '00000000-0000-0000-0000-000000000001' WHERE user_id IS NULL OR user_id = 'default';
CREATE INDEX IF NOT EXISTS idx_pg_journal_user_date ON journal_entries(user_id, entry_date DESC);
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'fk_journal_entries_user_id') THEN
        ALTER TABLE journal_entries
            ADD CONSTRAINT fk_journal_entries_user_id
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
    END IF;
END $$;
