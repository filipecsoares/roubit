import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type { Routine, TimeBlock, CurrentRoutineProgress, RoutineWithProgressResponse, BlockStatusType } from '@/types'

export const useRoutineStore = defineStore('routine', () => {
  const routine = ref<Routine | null>(null)
  const blocks = ref<TimeBlock[]>([])
  const progress = ref<CurrentRoutineProgress | null>(null)
  const completedBlockIds = ref<string[]>([])
  const skippedBlockIds = ref<string[]>([])
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
      completedBlockIds.value = data.completed_block_ids || []
      skippedBlockIds.value = data.skipped_block_ids || []
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
      try {
        await fetchActiveRoutine()
      } catch {
        // Offline: preserve optimistic creation
      }
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

    // Optimistic update
    if (status === 'completed') {
      if (!completedBlockIds.value.includes(blockId)) {
        completedBlockIds.value.push(blockId)
      }
      skippedBlockIds.value = skippedBlockIds.value.filter((id) => id !== blockId)
    } else if (status === 'skipped') {
      if (!skippedBlockIds.value.includes(blockId)) {
        skippedBlockIds.value.push(blockId)
      }
      completedBlockIds.value = completedBlockIds.value.filter((id) => id !== blockId)
    } else {
      completedBlockIds.value = completedBlockIds.value.filter((id) => id !== blockId)
      skippedBlockIds.value = skippedBlockIds.value.filter((id) => id !== blockId)
    }

    try {
      await apiClient.post(`/routines/blocks/${blockId}/complete`, { status, notes })
      try {
        await fetchActiveRoutine()
      } catch {
        // Offline: update local block status optimistically
        const block = blocks.value.find((b) => b.id === blockId)
        if (block && progress.value) {
          if (status === 'completed') {
            progress.value.completed_blocks += 1
          } else if (status === 'skipped') {
            progress.value.skipped_blocks += 1
          }
          if (progress.value.pending_blocks > 0) {
            progress.value.pending_blocks -= 1
          }
        }
      }
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
    completedBlockIds,
    skippedBlockIds,
    loading,
    error,
    fetchActiveRoutine,
    createRoutine,
    completeBlock,
  }
})
