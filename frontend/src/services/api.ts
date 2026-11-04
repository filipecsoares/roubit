import axios, { type InternalAxiosRequestConfig } from 'axios'
import { useAuthStore } from '@/stores/auth'
import { enqueueMutation, initOfflineSync } from './offlineQueue'

export const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1',
  headers: {
    'Content-Type': 'application/json',
  },
  withCredentials: true,
})

// Request Interceptor: Injects Bearer token and UUIDv4 Idempotency-Key on mutations
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    // Inject Authorization token from store or localStorage
    try {
      const authStore = useAuthStore()
      if (authStore.token) {
        config.headers.set('Authorization', `Bearer ${authStore.token}`)
      }
    } catch {
      const token = typeof localStorage !== 'undefined' ? localStorage.getItem('roubit_token') : null
      if (token) {
        config.headers.set('Authorization', `Bearer ${token}`)
      }
    }

    // Inject Idempotency-Key on state-mutating HTTP methods
    const method = config.method?.toLowerCase() || ''
    if (['post', 'put', 'patch', 'delete'].includes(method)) {
      if (!config.headers.get('Idempotency-Key')) {
        config.headers.set('Idempotency-Key', crypto.randomUUID())
      }
    }

    return config
  },
  (error) => Promise.reject(error),
)

// Response Interceptor: Catches 401 Unauthorized and intercepts offline network errors
apiClient.interceptors.response.use(
  (response) => response,
  async (error) => {
    const config = error.config as (InternalAxiosRequestConfig & { skipOfflineQueue?: boolean }) | undefined

    // 1. Check for offline or network reachability failures
    const isNetworkError =
      !error.response &&
      (typeof window !== 'undefined'
        ? !window.navigator.onLine ||
          error.code === 'ERR_NETWORK' ||
          error.message?.includes('Network Error') ||
          error.message?.includes('Failed to fetch')
        : false)

    if (isNetworkError && config && !config.skipOfflineQueue) {
      const method = (config.method?.toUpperCase() || '') as 'POST' | 'PUT' | 'DELETE' | 'PATCH'
      if (['POST', 'PUT', 'PATCH', 'DELETE'].includes(method)) {
        const idempotencyKey =
          (config.headers?.get?.('Idempotency-Key') as string) ||
          (config.headers?.['Idempotency-Key'] as string) ||
          crypto.randomUUID()

        let payloadData: any = config.data
        if (typeof config.data === 'string') {
          try {
            payloadData = JSON.parse(config.data)
          } catch {
            payloadData = config.data
          }
        }

        // Enqueue mutation with its original Idempotency-Key
        await enqueueMutation({
          url: config.url || '',
          method,
          payload: payloadData,
          idempotencyKey,
        })

        // Synthesize an optimistic response for UI stores
        let syntheticData: any = {
          success: true,
          offline: true,
          queued: true,
          message: 'Operação salva offline. Será sincronizada automaticamente.',
        }

        // Special tailored optimistic payloads for specific domains
        if (config.url?.includes('/habits/') && config.url?.includes('/checkin')) {
          const parts = config.url.split('/')
          const habitId = parts[parts.indexOf('habits') + 1] || ''
          syntheticData = {
            log: {
              id: crypto.randomUUID(),
              habit_id: habitId,
              log_date: payloadData?.date || new Date().toISOString().split('T')[0],
              actual_value: payloadData?.value || 1,
              is_completed: true,
              created_at: new Date().toISOString(),
              updated_at: new Date().toISOString(),
            },
            streak: {
              current_streak: 1,
              longest_streak: 1,
              is_active_today: true,
            },
            offline: true,
            queued: true,
          }
        }

        return Promise.resolve({
          data: syntheticData,
          status: 202,
          statusText: 'Accepted (Offline Queued)',
          headers: {},
          config,
        })
      }
    }

    // 2. Authentication failure (401 Unauthorized)
    if (error.response?.status === 401) {
      try {
        const authStore = useAuthStore()
        await authStore.logout()
      } catch {
        if (typeof localStorage !== 'undefined') {
          localStorage.removeItem('roubit_token')
        }
      }

      if (typeof window !== 'undefined' && window.location.pathname !== '/login') {
        window.location.href = `/login?redirect=${encodeURIComponent(window.location.pathname)}`
      }
    }

    return Promise.reject(error)
  },
)

// Initialize offline auto-sync listeners
initOfflineSync(apiClient)

export default apiClient
