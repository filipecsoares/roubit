import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { apiClient } from '../api'
import { useAuthStore } from '@/stores/auth'

describe('apiClient interceptors', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('attaches Authorization header when user is authenticated in store', async () => {
    const authStore = useAuthStore()
    authStore.token = 'mock-jwt-token-xyz'

    const config = await (apiClient.interceptors.request as any).handlers[0].fulfilled({
      method: 'get',
      headers: new Map(),
    })

    expect(config.headers.get('Authorization')).toBe('Bearer mock-jwt-token-xyz')
  })

  it('generates a valid UUIDv4 Idempotency-Key header on mutating requests (POST)', async () => {
    const config = await (apiClient.interceptors.request as any).handlers[0].fulfilled({
      method: 'post',
      headers: new Map(),
    })

    const key = config.headers.get('Idempotency-Key')
    expect(key).toBeDefined()
    // Verify standard UUIDv4 format (8-4-4-4-12 hex characters)
    expect(key).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i)
  })

  it('does NOT generate Idempotency-Key header on GET requests', async () => {
    const config = await (apiClient.interceptors.request as any).handlers[0].fulfilled({
      method: 'get',
      headers: new Map(),
    })

    expect(config.headers.get('Idempotency-Key')).toBeUndefined()
  })

  it('preserves existing Idempotency-Key header if caller provided one explicitly', async () => {
    const headers = new Map()
    headers.set('Idempotency-Key', 'explicit-custom-key-123')

    const config = await (apiClient.interceptors.request as any).handlers[0].fulfilled({
      method: 'post',
      headers,
    })

    expect(config.headers.get('Idempotency-Key')).toBe('explicit-custom-key-123')
  })

  it('handles 401 response by logging out and purging local storage token', async () => {
    const authStore = useAuthStore()
    authStore.token = 'old-token'
    localStorage.setItem('roubit_token', 'old-token')

    const responseInterceptor = (apiClient.interceptors.response as any).handlers[0].rejected

    const error401 = {
      response: {
        status: 401,
        data: { title: 'Unauthorized' },
      },
    }

    await expect(responseInterceptor(error401)).rejects.toEqual(error401)
    expect(authStore.token).toBeNull()
    expect(localStorage.getItem('roubit_token')).toBeNull()
  })
})
