import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '@/services/api'
import type { JournalMetadata, EncryptedJournalResponse, SaveEncryptedJournalRequest } from '@/types'

export const useJournalStore = defineStore('journal', () => {
  const metadataList = ref<JournalMetadata[]>([])
  const currentEntry = ref<EncryptedJournalResponse | null>(null)
  const isUnlocked = ref(false)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchMetadata(startDate?: string, endDate?: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<JournalMetadata[]>('/journal/metadata', {
        params: { start_date: startDate, end_date: endDate },
      })
      metadataList.value = data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao carregar histórico do diário'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function fetchEntry(id: string): Promise<EncryptedJournalResponse> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.get<EncryptedJournalResponse>(`/journal/${id}`)
      currentEntry.value = data
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao buscar entrada cifrada'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function saveEncryptedEntry(payload: SaveEncryptedJournalRequest): Promise<EncryptedJournalResponse> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<EncryptedJournalResponse>('/journal', payload)
      currentEntry.value = data
      await fetchMetadata()
      return data
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao salvar entrada cifrada'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function deleteEntry(id: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      await apiClient.delete(`/journal/${id}`)
      if (currentEntry.value?.id === id) {
        currentEntry.value = null
      }
      await fetchMetadata()
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Erro ao excluir entrada'
      throw err
    } finally {
      loading.value = false
    }
  }

  function setUnlocked(unlocked: boolean): void {
    isUnlocked.value = unlocked
  }

  return {
    metadataList,
    currentEntry,
    isUnlocked,
    loading,
    error,
    fetchMetadata,
    fetchEntry,
    saveEncryptedEntry,
    deleteEntry,
    setUnlocked,
  }
})
