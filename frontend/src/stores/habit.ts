import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type {
  HabitWithStreakResponse,
  Habit,
  CheckinResponse,
  HabitKind,
  HabitAnalyticsResponse,
} from '@/types'

export const useHabitStore = defineStore('habit', () => {
  const habits = ref<HabitWithStreakResponse[]>([])
  const analytics = ref<HabitAnalyticsResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  function updateBadge(): void {
    if (typeof navigator !== 'undefined' && 'setAppBadge' in navigator) {
      const pendingCount = habits.value.filter((h) => !h.habit?.is_archived && !h.streak?.is_active_today).length
      if (pendingCount > 0) {
        navigator.setAppBadge(pendingCount).catch(() => {})
      } else {
        navigator.clearAppBadge().catch(() => {})
      }
    }
  }

  async function fetchHabits(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<HabitWithStreakResponse[]>('/habits')
      habits.value = data
      updateBadge()
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao listar hábitos'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function fetchAnalytics(): Promise<HabitAnalyticsResponse> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<HabitAnalyticsResponse>('/habits/analytics')
      analytics.value = data
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao carregar métricas de hábitos'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function createHabit(payload: {
    name: string
    description?: string
    habit_type: HabitKind
    target_value: number
    unit?: string
    frequency_days?: number
    color_hex?: string
  }): Promise<Habit> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<Habit>('/habits', payload)
      try {
        await fetchHabits()
      } catch {
        // Offline: preserve optimistic creation
      }
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao criar hábito'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function checkin(
    habitId: string,
    date: string,
    value: number,
    isIncrement: boolean = false,
  ): Promise<CheckinResponse> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<CheckinResponse>(`/habits/${habitId}/checkin`, {
        date,
        value,
        is_increment: isIncrement,
      })
      // Update local habit streak and today_value in-place
      const idx = habits.value.findIndex((h) => h.habit.id === habitId)
      if (idx !== -1 && data) {
        if (data.streak) {
          habits.value[idx].streak = data.streak
        }
        if (data.log) {
          habits.value[idx].today_value = data.log.actual_value
        }
        updateBadge()
      }
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao registrar check-in'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function archiveHabit(habitId: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      await apiClient.post(`/habits/${habitId}/archive`)
      habits.value = habits.value.filter((h) => h.habit.id !== habitId)
      updateBadge()
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao arquivar hábito'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function deleteHabit(habitId: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      await apiClient.delete(`/habits/${habitId}`)
      habits.value = habits.value.filter((h) => h.habit.id !== habitId)
      updateBadge()
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao excluir hábito'
      throw err
    } finally {
      loading.value = false
    }
  }

  return {
    habits,
    analytics,
    loading,
    error,
    fetchHabits,
    fetchAnalytics,
    createHabit,
    checkin,
    archiveHabit,
    deleteHabit,
  }
})
