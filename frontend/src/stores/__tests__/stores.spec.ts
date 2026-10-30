import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { apiClient } from '@/services/api'
import { useAuthStore } from '../auth'
import { useRoutineStore } from '../routine'
import { useHabitStore } from '../habit'
import { useOkrStore } from '../okr'
import { useJournalStore } from '../journal'

describe('Pinia Core Domain Stores', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.restoreAllMocks()
  })

  describe('useAuthStore', () => {
    it('initializes with default state from localStorage', () => {
      localStorage.setItem('roubit_token', 'initial-token-123')
      const store = useAuthStore()
      expect(store.token).toBe('initial-token-123')
      expect(store.isAuthenticated).toBe(true)
    })

    it('login sets token and user, storing token in localStorage', async () => {
      const mockAuthResponse = {
        token: 'new-jwt-token',
        user: { id: 'u1', email: 'test@roubit.app', created_at: '2026-10-30T00:00:00Z' },
      }
      vi.spyOn(apiClient, 'post').mockResolvedValueOnce({ data: mockAuthResponse } as any)

      const store = useAuthStore()
      await store.login('test@roubit.app', 'password123')

      expect(store.token).toBe('new-jwt-token')
      expect(store.user?.email).toBe('test@roubit.app')
      expect(localStorage.getItem('roubit_token')).toBe('new-jwt-token')
    })

    it('logout clears state and removes token from localStorage', () => {
      const store = useAuthStore()
      store.token = 'active-token'
      localStorage.setItem('roubit_token', 'active-token')

      store.logout()
      expect(store.token).toBeNull()
      expect(store.user).toBeNull()
      expect(localStorage.getItem('roubit_token')).toBeNull()
    })
  })

  describe('useRoutineStore', () => {
    it('fetchActiveRoutine updates routine, blocks, and progress state', async () => {
      const mockData = {
        routine: { id: 'r1', name: 'Morning Focus', is_active: true },
        blocks: [{ id: 'b1', name: 'Meditation', start_time: '08:00', end_time: '08:30' }],
        progress: { adherence_percentage: 100, total_blocks: 1, completed_blocks: 1 },
      }
      vi.spyOn(apiClient, 'get').mockResolvedValueOnce({ data: mockData } as any)

      const store = useRoutineStore()
      await store.fetchActiveRoutine()

      expect(store.routine?.name).toBe('Morning Focus')
      expect(store.blocks.length).toBe(1)
      expect(store.progress?.adherence_percentage).toBe(100)
    })
  })

  describe('useHabitStore', () => {
    it('checkin updates streak in place', async () => {
      const habitId = 'h1'
      const store = useHabitStore()
      store.habits = [
        {
          habit: { id: habitId, name: 'Water', habit_type: 'numeric', target_value: 2.5 } as any,
          streak: { current_streak: 5, longest_streak: 10, is_active_today: false },
        },
      ]

      const checkinResponse = {
        log: { id: 'l1', habit_id: habitId, log_date: '2026-10-30', actual_value: 2.5, is_completed: true },
        streak: { current_streak: 6, longest_streak: 10, is_active_today: true },
      }
      vi.spyOn(apiClient, 'post').mockResolvedValueOnce({ data: checkinResponse } as any)

      await store.checkin(habitId, '2026-10-30', 2.5)

      expect(store.habits[0].streak.current_streak).toBe(6)
      expect(store.habits[0].streak.is_active_today).toBe(true)
    })
  })

  describe('useOkrStore', () => {
    it('fetchOkrs queries objectives by quarter', async () => {
      const mockOkrs = [
        {
          objective: { id: 'o1', title: 'Ship v1.0', quarter: 'Q4-2026', progress_percentage: 80 },
          key_results: [{ id: 'kr1', title: 'Complete testing', target_value: 100, current_value: 80 }],
        },
      ]
      vi.spyOn(apiClient, 'get').mockResolvedValueOnce({ data: mockOkrs } as any)

      const store = useOkrStore()
      await store.fetchOkrs('Q4-2026')

      expect(store.objectives.length).toBe(1)
      expect(store.objectives[0].objective.title).toBe('Ship v1.0')
    })
  })

  describe('useJournalStore', () => {
    it('fetchMetadata retrieves list of metadata omitting ciphertext', async () => {
      const mockMeta = [
        { id: 'j1', entry_date: '2026-10-30', mood_score: 5, tags: ['focus'], ciphertext_bytes_len: 48 },
      ]
      vi.spyOn(apiClient, 'get').mockResolvedValueOnce({ data: mockMeta } as any)

      const store = useJournalStore()
      await store.fetchMetadata()

      expect(store.metadataList.length).toBe(1)
      expect(store.metadataList[0].mood_score).toBe(5)
    })
  })
})
