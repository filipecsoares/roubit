import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type { ObjectiveWithKeyResults } from '@/types'

export const useOkrStore = defineStore('okr', () => {
  const objectives = ref<ObjectiveWithKeyResults[]>([])
  const quarter = ref<string>('Q4-2026')
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchOkrs(targetQuarter?: string): Promise<void> {
    loading.value = true
    error.value = null
    const q = targetQuarter || quarter.value
    try {
      const { data } = await apiClient.get<ObjectiveWithKeyResults[]>('/okrs', {
        params: q ? { quarter: q } : undefined,
      })
      objectives.value = data
      if (targetQuarter) {
        quarter.value = targetQuarter
      }
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao listar OKRs'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function createObjective(payload: {
    title: string
    description?: string
    quarter?: string
    start_date?: string
    end_date?: string
    key_results?: Array<{
      title: string
      initial_value?: number
      target_value: number
      unit?: string
      weight?: number
      linked_habit_id?: string
    }>
  }): Promise<ObjectiveWithKeyResults> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<ObjectiveWithKeyResults>('/okrs', payload)
      await fetchOkrs(payload.quarter || quarter.value)
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao criar objetivo'
      throw err
    } finally {
      loading.value = false
    }
  }

  return {
    objectives,
    quarter,
    loading,
    error,
    fetchOkrs,
    createObjective,
  }
})
