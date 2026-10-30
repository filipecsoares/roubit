import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type { Routine, TimeBlock, CurrentRoutineProgress, RoutineWithProgressResponse, BlockStatusType } from '@/types'

export const useRoutineStore = defineStore('routine', () => {
  const routine = ref<Routine | null>(null)
  const blocks = ref<TimeBlock[]>([])
  const progress = ref<CurrentRoutineProgress | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchActiveRoutine(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<RoutineWithProgressResponse>('/routines')
      routine.value = data.routine
      blocks.value = data.blocks
      progress.value = data.progress
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao carregar rotina diária'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function createRoutine(payload: {
    name: string
    description?: string
    is_active?: boolean
    blocks: Array<{
      name: string
      start_time: string
      end_time: string
      category?: string
      color_hex?: string
      order_index?: number
    }>
  }): Promise<Routine> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<Routine>('/routines', payload)
      await fetchActiveRoutine()
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao criar rotina'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function completeBlock(blockId: string, status: BlockStatusType = 'completed', notes?: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      await apiClient.post(`/routines/blocks/${blockId}/complete`, { status, notes })
      await fetchActiveRoutine()
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao atualizar bloco'
      throw err
    } finally {
      loading.value = false
    }
  }

  return {
    routine,
    blocks,
    progress,
    loading,
    error,
    fetchActiveRoutine,
    createRoutine,
    completeBlock,
  }
})
