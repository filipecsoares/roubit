-- 0003_create_okrs.sql
-- Goal & OKR Alignment Engine SQLite Schema (RF03)

CREATE TABLE IF NOT EXISTS objectives (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    quarter TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    progress_percentage REAL NOT NULL DEFAULT 0.0 CHECK(progress_percentage >= 0.0 AND progress_percentage <= 100.0),
    is_completed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_objectives_quarter ON objectives(quarter);
CREATE INDEX IF NOT EXISTS idx_objectives_completed ON objectives(is_completed);

CREATE TABLE IF NOT EXISTS key_results (
    id TEXT PRIMARY KEY NOT NULL,
    objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE CASCADE,
    linked_habit_id TEXT REFERENCES habits(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    initial_value REAL NOT NULL DEFAULT 0.0,
    target_value REAL NOT NULL,
    current_value REAL NOT NULL DEFAULT 0.0,
    unit TEXT NOT NULL DEFAULT '',
    weight REAL NOT NULL DEFAULT 1.0 CHECK(weight > 0.0),
    progress_percentage REAL NOT NULL DEFAULT 0.0 CHECK(progress_percentage >= 0.0 AND progress_percentage <= 100.0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_key_results_objective_id ON key_results(objective_id);
CREATE INDEX IF NOT EXISTS idx_key_results_habit_id ON key_results(linked_habit_id);
