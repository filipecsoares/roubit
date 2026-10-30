-- 0001_create_routines.sql
-- Routine Management (Time Blocks) PostgreSQL Schema

CREATE TABLE IF NOT EXISTS routines (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    name TEXT NOT NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS time_blocks (
    id TEXT PRIMARY KEY NOT NULL,
    routine_id TEXT NOT NULL REFERENCES routines(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    start_time TEXT NOT NULL, -- HH:MM or HH:MM:SS format
    end_time TEXT NOT NULL,   -- HH:MM or HH:MM:SS format
    category TEXT NOT NULL DEFAULT 'General',
    color_hex TEXT NOT NULL DEFAULT '#6366F1',
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pg_time_blocks_routine_id ON time_blocks(routine_id);
CREATE INDEX IF NOT EXISTS idx_pg_time_blocks_start_time ON time_blocks(start_time);

CREATE TABLE IF NOT EXISTS daily_routine_logs (
    id TEXT PRIMARY KEY NOT NULL,
    time_block_id TEXT NOT NULL REFERENCES time_blocks(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'in_progress', 'completed', 'skipped')),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    notes TEXT,
    UNIQUE(time_block_id, date)
);

CREATE INDEX IF NOT EXISTS idx_pg_daily_routine_logs_date ON daily_routine_logs(date);
