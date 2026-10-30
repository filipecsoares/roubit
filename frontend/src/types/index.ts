/**
 * Roubit Domain Types & API DTOs
 * Mirrored directly from Rust backend entities and RFC 7807 problem details
 */

export interface User {
  id: string
  email: string
  created_at: string
}

export interface AuthResponse {
  token: string
  user: User
}

export interface ProblemDetails {
  type: string
  title: string
  status: number
  detail: string
  instance?: string
}

// Routine Management (RF01)
export type BlockStatusType = 'pending' | 'in_progress' | 'completed' | 'skipped'

export interface TimeBlock {
  id: string
  routine_id: string
  name: string
  start_time: string // HH:MM:SS
  end_time: string   // HH:MM:SS
  category: string
  color_hex: string
  order_index: number
  created_at: string
}

export interface Routine {
  id: string
  name: string
  description?: string
  is_active: boolean
  created_at: string
  updated_at: string
}

export interface CurrentRoutineProgress {
  total_blocks: number
  completed_blocks: number
  in_progress_blocks: number
  skipped_blocks: number
  pending_blocks: number
  adherence_percentage: number
  active_block: TimeBlock | null
  next_block: TimeBlock | null
  seconds_until_next_block: number | null
}

export interface RoutineWithProgressResponse {
  routine: Routine | null
  blocks: TimeBlock[]
  progress: CurrentRoutineProgress | null
}

// Habit Tracking (RF02)
export type HabitKind = 'binary' | 'numeric' | 'timer'

export interface Habit {
  id: string
  name: string
  description?: string
  habit_type: HabitKind
  target_value: number
  unit?: string
  frequency_days: number
  is_archived: boolean
  color_hex: string
  created_at: string
  updated_at: string
}

export interface StreakMetrics {
  current_streak: number
  longest_streak: number
  is_active_today: boolean
}

export interface HabitWithStreakResponse {
  habit: Habit
  streak: StreakMetrics
}

export interface HabitLog {
  id: string
  habit_id: string
  log_date: string
  actual_value: number
  is_completed: boolean
  created_at: string
  updated_at: string
}

export interface CheckinResponse {
  log: HabitLog
  streak: StreakMetrics
}

// Objectives & OKRs (RF03)
export interface KeyResult {
  id: string
  objective_id: string
  linked_habit_id?: string
  title: string
  initial_value: number
  target_value: number
  current_value: number
  unit: string
  weight: number
  progress_percentage: number
  created_at: string
  updated_at: string
}

export interface Objective {
  id: string
  title: string
  description?: string
  quarter: string
  start_date: string
  end_date: string
  progress_percentage: number
  is_completed: boolean
  created_at: string
  updated_at: string
}

export interface ObjectiveWithKeyResults {
  objective: Objective
  key_results: KeyResult[]
}

// Encrypted Journaling (RF04)
export interface JournalMetadata {
  id: string
  entry_date: string
  mood_score?: number
  tags: string[]
  ciphertext_bytes_len: number
  created_at: string
  updated_at: string
}

export interface EncryptedJournalResponse {
  id: string
  entry_date: string
  mood_score?: number
  tags: string[]
  salt_base64: string
  nonce_base64: string
  ciphertext_base64: string
}

export interface SaveEncryptedJournalRequest {
  id?: string
  entry_date: string
  mood_score?: number
  tags?: string[]
  salt_base64: string
  nonce_base64: string
  ciphertext_base64: string
}
