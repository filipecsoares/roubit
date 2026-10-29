-- 0002_create_habits.sql
-- Habit Tracker SQLite Schema (RF02)

CREATE TABLE IF NOT EXISTS habits (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    habit_type TEXT NOT NULL CHECK(habit_type IN ('binary', 'numeric', 'timer')),
    target_value REAL NOT NULL DEFAULT 1.0,
    unit TEXT,
    frequency_days INTEGER NOT NULL DEFAULT 127, -- bitmask for 7 days (1=Mon, 2=Tue, 4=Wed, 8=Thu, 16=Fri, 32=Sat, 64=Sun; 127=Daily)
    color_hex TEXT NOT NULL DEFAULT '#10B981',
    is_archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_habits_is_archived ON habits(is_archived);

CREATE TABLE IF NOT EXISTS habit_logs (
    id TEXT PRIMARY KEY NOT NULL,
    habit_id TEXT NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
    log_date TEXT NOT NULL, -- YYYY-MM-DD
    actual_value REAL NOT NULL DEFAULT 0.0 CHECK(actual_value >= 0.0),
    is_completed INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    logged_at TEXT NOT NULL,
    UNIQUE(habit_id, log_date)
);

CREATE INDEX IF NOT EXISTS idx_habit_logs_habit_date ON habit_logs(habit_id, log_date DESC);
CREATE INDEX IF NOT EXISTS idx_habit_logs_date ON habit_logs(log_date);
