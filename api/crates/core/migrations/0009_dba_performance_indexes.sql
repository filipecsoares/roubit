-- 0009_dba_performance_indexes.sql
-- DBA Optimization: Covering & Composite Indexes for High-Frequency Access Paths (SQLite)

-- 1. Accelerate idempotency key purging (eliminates full table scan on DELETE)
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_created_at ON idempotency_keys(created_at);

-- 2. Covering index for time_blocks retrieval by routine_id with deterministic order
CREATE INDEX IF NOT EXISTS idx_time_blocks_routine_order ON time_blocks(routine_id, order_index, start_time);

-- 3. Composite index for atomic streak calculation: WHERE habit_id = ? AND is_completed = 1 AND log_date < ?
CREATE INDEX IF NOT EXISTS idx_habit_logs_streak_calc ON habit_logs(habit_id, is_completed, log_date DESC);

-- 4. Covering index for daily completion check: WHERE log_date = ? AND is_completed = 1
CREATE INDEX IF NOT EXISTS idx_habit_logs_date_completed ON habit_logs(log_date, is_completed, habit_id);

-- 5. Covering index for listing key results ordered by creation under an objective
CREATE INDEX IF NOT EXISTS idx_key_results_obj_created ON key_results(objective_id, created_at ASC);

-- 6. Index for batch daily routine logs lookup: WHERE date = ? AND time_block_id = ?
CREATE INDEX IF NOT EXISTS idx_daily_routine_logs_block_date ON daily_routine_logs(time_block_id, date);

-- 7. Composite index for journal entry range filtering and sorting
CREATE INDEX IF NOT EXISTS idx_journal_user_entry_created ON journal_entries(user_id, entry_date DESC, created_at DESC);
