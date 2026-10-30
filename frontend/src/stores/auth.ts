import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiClient } from '@/services/api'
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
      logout()
    } finally {
      loading.value = false
    }
  }

  function logout(): void {
    token.value = null
    user.value = null
    localStorage.removeItem('roubit_token')
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
