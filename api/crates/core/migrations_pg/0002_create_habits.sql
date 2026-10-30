-- 0002_create_habits.sql
-- Habit Tracker PostgreSQL Schema (RF02)

CREATE TABLE IF NOT EXISTS habits (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    name TEXT NOT NULL,
    description TEXT,
    habit_type TEXT NOT NULL CHECK(habit_type IN ('binary', 'numeric', 'timer')),
    target_value DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    unit TEXT,
    frequency_days INTEGER NOT NULL DEFAULT 127,
    color_hex TEXT NOT NULL DEFAULT '#10B981',
    is_archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pg_habits_is_archived ON habits(is_archived);

CREATE TABLE IF NOT EXISTS habit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    habit_id TEXT NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    log_date DATE NOT NULL,
    actual_value DOUBLE PRECISION NOT NULL DEFAULT 0.0 CHECK(actual_value >= 0.0),
    is_completed BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    logged_at TIMESTAMPTZ NOT NULL,
    UNIQUE(habit_id, log_date)
);

CREATE INDEX IF NOT EXISTS idx_pg_habit_logs_habit_date ON habit_logs(habit_id, log_date DESC);
CREATE INDEX IF NOT EXISTS idx_pg_habit_logs_date ON habit_logs(log_date);
