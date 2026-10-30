import axios, { type InternalAxiosRequestConfig } from 'axios'
import { useAuthStore } from '@/stores/auth'

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
      const token = localStorage.getItem('roubit_token')
      if (token) {
        config.headers.set('Authorization', `Bearer ${token}`)
      }
    }

    // Inject Idempotency-Key on state-mutating HTTP methods
    const method = config.method?.toLowerCase() || ''
    if (['post', 'put', 'patch'].includes(method)) {
      if (!config.headers.get('Idempotency-Key')) {
        config.headers.set('Idempotency-Key', crypto.randomUUID())
      }
    }

    return config
  },
  (error) => Promise.reject(error),
)

// Response Interceptor: Catches 401 Unauthorized and invalidates session
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      try {
        const authStore = useAuthStore()
        authStore.logout()
      } catch {
        localStorage.removeItem('roubit_token')
      }

      if (window.location.pathname !== '/login') {
        window.location.href = `/login?redirect=${encodeURIComponent(window.location.pathname)}`
      }
    }
    return Promise.reject(error)
  },
)

export default apiClient
