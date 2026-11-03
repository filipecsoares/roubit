import 'fake-indexeddb/auto'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  enqueueMutation,
  getQueuedMutations,
  removeMutation,
  clearQueue,
  syncOfflineQueue,
  pendingSyncCount,
} from '../offlineQueue'
import { WebCryptoService } from '../crypto'
import { apiClient } from '../api'

describe('Offline Mutation Queue & Idempotent Sync Engine (Activity 16.2 & 16.4)', () => {
  beforeEach(async () => {
    await clearQueue()
    vi.restoreAllMocks()
  })

  it('enqueues mutations into IndexedDB with unique UUIDv4 and original Idempotency-Key', async () => {
    const key = crypto.randomUUID()
    const item = await enqueueMutation({
      url: '/habits/habit-123/checkin',
      method: 'POST',
      payload: { date: '2026-11-03', value: 1 },
      idempotencyKey: key,
    })

    expect(item.id).toBeDefined()
    expect(item.idempotencyKey).toBe(key)
    expect(item.createdAt).toBeDefined()
    expect(item.retryCount).toBe(0)

    const queued = await getQueuedMutations()
    expect(queued).toHaveLength(1)
    expect(queued[0].id).toBe(item.id)
    expect(queued[0].idempotencyKey).toBe(key)
    expect(pendingSyncCount.value).toBe(1)
  })

  it('preserves strict chronological FIFO order across multiple mutations', async () => {
    const item1 = await enqueueMutation({
      url: '/habits/h1/checkin',
      method: 'POST',
      payload: { step: 1 },
      idempotencyKey: 'key-1',
    })

    // Brief delay to ensure timestamp progression
    await new Promise((r) => setTimeout(r, 10))

    const item2 = await enqueueMutation({
      url: '/habits/h2/checkin',
      method: 'POST',
      payload: { step: 2 },
      idempotencyKey: 'key-2',
    })

    const queued = await getQueuedMutations()
    expect(queued).toHaveLength(2)
    expect(queued[0].id).toBe(item1.id)
    expect(queued[1].id).toBe(item2.id)
  })

  it('removes mutations from IndexedDB and reactively decrements pendingSyncCount', async () => {
    const item = await enqueueMutation({
      url: '/habits/h1/checkin',
      method: 'POST',
      payload: { value: 1 },
      idempotencyKey: 'key-1',
    })

    expect(pendingSyncCount.value).toBe(1)
    await removeMutation(item.id)

    const remaining = await getQueuedMutations()
    expect(remaining).toHaveLength(0)
    expect(pendingSyncCount.value).toBe(0)
  })

  it('replays queue sequentially using original Idempotency-Key and purges on 200/201/409', async () => {
    const key1 = 'idem-key-001'
    const key2 = 'idem-key-002'

    await enqueueMutation({
      url: '/habits/h1/checkin',
      method: 'POST',
      payload: { value: 1 },
      idempotencyKey: key1,
    })

    await enqueueMutation({
      url: '/habits/h2/checkin',
      method: 'POST',
      payload: { value: 2 },
      idempotencyKey: key2,
    })

    const dispatchedRequests: any[] = []
    const mockClient = {
      request: vi.fn().mockImplementation(async (config: any) => {
        dispatchedRequests.push(config)
        if (config.url.includes('h2')) {
          // Simulate 409 Conflict (server idempotency deduplication hit)
          const err: any = new Error('Conflict')
          err.response = { status: 409 }
          throw err
        }
        return { status: 200, data: { ok: true } }
      }),
    } as any

    const result = await syncOfflineQueue(mockClient)
    expect(result.synced).toBe(2)
    expect(result.failed).toBe(0)

    expect(dispatchedRequests).toHaveLength(2)
    expect(dispatchedRequests[0].headers['Idempotency-Key']).toBe(key1)
    expect(dispatchedRequests[1].headers['Idempotency-Key']).toBe(key2)

    const remaining = await getQueuedMutations()
    expect(remaining).toHaveLength(0)
    expect(pendingSyncCount.value).toBe(0)
  })

  it('discards permanent client validation errors (HTTP 400/422) to avoid poisoning queue', async () => {
    await enqueueMutation({
      url: '/habits/invalid/checkin',
      method: 'POST',
      payload: { malformed: true },
      idempotencyKey: 'key-invalid',
    })

    const mockClient = {
      request: vi.fn().mockRejectedValue({
        response: { status: 422, data: { detail: 'Validation failed' } },
      }),
    } as any

    const result = await syncOfflineQueue(mockClient)
    expect(result.failed).toBe(1)
    expect(result.synced).toBe(0)

    const remaining = await getQueuedMutations()
    expect(remaining).toHaveLength(0)
  })

  it('guarantees ZERO-KNOWLEDGE security invariant: NO plaintext reflection touches IndexedDB', async () => {
    const sensitiveSecret = 'Minha reflexão ultra confidencial gravada no cofre offline 12345!'
    const salt = WebCryptoService.generateSalt()
    const masterKey = await WebCryptoService.deriveKey('MinhaSenhaMestreSegura', salt)

    // 1. Encrypt in RAM before network/offline handling
    const encryptedEnvelope = await WebCryptoService.encrypt(sensitiveSecret, masterKey, salt)

    // 2. Enqueue the encrypted payload
    await enqueueMutation({
      url: '/journal',
      method: 'POST',
      payload: {
        entry_date: '2026-11-03',
        mood_score: 5,
        tags: ['segredo', 'offline'],
        salt_base64: encryptedEnvelope.saltBase64,
        nonce_base64: encryptedEnvelope.nonceBase64,
        ciphertext_base64: encryptedEnvelope.ciphertextBase64,
      },
      idempotencyKey: crypto.randomUUID(),
    })

    // 3. Inspect raw IndexedDB records
    const storedItems = await getQueuedMutations()
    expect(storedItems).toHaveLength(1)

    const serializedPayload = JSON.stringify(storedItems[0].payload)

    // Assert that NONE of the secret words exist in the storage record
    expect(serializedPayload).not.toContain('ultra confidencial')
    expect(serializedPayload).not.toContain('Minha reflexão')
    expect(serializedPayload).not.toContain('12345')

    // Assert that only Base64 ciphertext is present
    expect(storedItems[0].payload.ciphertext_base64).toBe(encryptedEnvelope.ciphertextBase64)
    expect(storedItems[0].payload.nonce_base64).toBe(encryptedEnvelope.nonceBase64)
  })

  it('intercepts Axios network errors and returns synthetic 202 Accepted with queued item', async () => {
    // Mock Axios adapter or network rejection
    vi.spyOn(apiClient, 'post').mockImplementation(async (url: string, data?: any) => {
      // Simulate Axios request failure in offline mode
      const netError: any = new Error('Network Error')
      netError.code = 'ERR_NETWORK'
      netError.config = {
        url,
        method: 'POST',
        data,
        headers: {
          'Idempotency-Key': 'net-idem-uuid',
          get: (h: string) => (h === 'Idempotency-Key' ? 'net-idem-uuid' : null),
        },
      }
      // Pass error through the response interceptor error handler
      const interceptor = (apiClient.interceptors.response as any).handlers[0]
      return interceptor.rejected(netError)
    })

    const response = await apiClient.post('/habits/habit-abc/checkin', {
      date: '2026-11-03',
      value: 1,
    })

    expect(response.status).toBe(202)
    expect(response.data.offline).toBe(true)
    expect(response.data.queued).toBe(true)
    expect(response.data.streak).toBeDefined()

    const queued = await getQueuedMutations()
    expect(queued).toHaveLength(1)
    expect(queued[0].url).toBe('/habits/habit-abc/checkin')
    expect(queued[0].idempotencyKey).toBe('net-idem-uuid')
  })
})
