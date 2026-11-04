import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiClient } from '@/services/api'
import { clearQueue } from '@/services/offlineQueue'
import { useVaultStore } from '@/stores/vault'
import { useNotificationStore } from '@/stores/notification'
import { useHabitStore } from '@/stores/habit'
import { useRoutineStore } from '@/stores/routine'
import { useOkrStore } from '@/stores/okr'
import { useJournalStore } from '@/stores/journal'
import { useFocusStore } from '@/stores/focus'
import type { User, AuthResponse } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('roubit_token'))
  const user = ref<User | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value)

  async function login(email: string, password: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<AuthResponse>('/auth/login', { email, password })
      token.value = data.token
      user.value = data.user
      localStorage.setItem('roubit_token', data.token)
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Falha ao efetuar login'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function register(email: string, password: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const { data } = await apiClient.post<AuthResponse>('/auth/register', { email, password })
      token.value = data.token
      user.value = data.user
      localStorage.setItem('roubit_token', data.token)
    } catch (err: any) {
      error.value = err.response?.data?.detail || 'Falha ao criar conta'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function fetchMe(): Promise<void> {
    if (!token.value) return
    loading.value = true
    try {
      const { data } = await apiClient.get<User>('/auth/me')
      user.value = data
    } catch (err) {
      await logout()
    } finally {
      loading.value = false
    }
  }

  async function logout(): Promise<void> {
    token.value = null
    user.value = null
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('roubit_token')
    }

    try {
      await apiClient.post('/auth/logout', {}, { skipOfflineQueue: true } as any)
    } catch {
      // Ignore network errors when logging out while offline
    }

    // 1. Wipe masterKey and derived keys from RAM (Zero-Knowledge Memory Safety)
    try {
      const vaultStore = useVaultStore()
      vaultStore.lock()
    } catch {
      // Ignore if vault store was not instantiated
    }

    // 2. Clear IndexedDB offline mutation queue to prevent cross-account replay attacks
    try {
      await clearQueue()
    } catch {
      // Ignore queue clearance error
    }

    // 3. Disconnect SSE stream and clear recent notifications
    try {
      const notificationStore = useNotificationStore()
      notificationStore.disconnectSSE()
      notificationStore.clearAll()
    } catch {
      // Ignore notification store errors
    }

    // 4. Scrub in-memory domain stores to avoid leaking data across user sessions
    try {
      const habitStore = useHabitStore()
      habitStore.habits = []
    } catch {}

    try {
      const routineStore = useRoutineStore()
      routineStore.routine = null
      routineStore.blocks = []
      routineStore.progress = null
    } catch {}

    try {
      const okrStore = useOkrStore()
      okrStore.objectives = []
    } catch {}

    try {
      const journalStore = useJournalStore()
      journalStore.metadataList = []
      journalStore.currentEntry = null
      journalStore.isUnlocked = false
    } catch {}

    try {
      const focusStore = useFocusStore()
      focusStore.resetTimer()
      focusStore.closeFocusModal()
    } catch {}
  }

  return {
    token,
    user,
    loading,
    error,
    isAuthenticated,
    login,
    register,
    fetchMe,
    logout,
  }
})
