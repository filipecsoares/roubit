-- 0008_habit_streak_denormalization.sql
-- Habit Streak Denormalization & Schema Update (RF02, RNF04)

ALTER TABLE habits ADD COLUMN current_streak INTEGER NOT NULL DEFAULT 0;
ALTER TABLE habits ADD COLUMN longest_streak INTEGER NOT NULL DEFAULT 0;

-- Backfill initial streaks from historical completed logs
UPDATE habits
SET current_streak = COALESCE((
    SELECT COUNT(*)
    FROM habit_logs
    WHERE habit_logs.habit_id = habits.id AND habit_logs.is_completed = 1
), 0),
longest_streak = COALESCE((
    SELECT COUNT(*)
    FROM habit_logs
    WHERE habit_logs.habit_id = habits.id AND habit_logs.is_completed = 1
), 0);
