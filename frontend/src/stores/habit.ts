import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type { HabitWithStreakResponse, Habit, CheckinResponse, HabitKind } from '@/types'

export const useHabitStore = defineStore('habit', () => {
  const habits = ref<HabitWithStreakResponse[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchHabits(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<HabitWithStreakResponse[]>('/habits')
      habits.value = data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao listar hábitos'
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
      await fetchHabits()
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
      // Update local habit streak in-place
      const idx = habits.value.findIndex((h) => h.habit.id === habitId)
      if (idx !== -1) {
        habits.value[idx].streak = data.streak
      }
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao registrar check-in'
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
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao excluir hábito'
      throw err
    } finally {
      loading.value = false
    }
  }

  return {
    habits,
    loading,
    error,
    fetchHabits,
    createHabit,
    checkin,
    deleteHabit,
  }
})
