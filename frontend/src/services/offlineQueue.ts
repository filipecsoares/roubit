/**
 * Offline Mutation Queue & Idempotent Sync Engine (Feature 16 / Activity 16.2)
 *
 * Persists mutating requests (habits checkin, routine completions, encrypted journal)
 * into IndexedDB when client is offline or network fails. Replays them sequentially in FIFO order
 * with their original Idempotency-Key when connectivity is restored.
 *
 * ZERO-KNOWLEDGE INVARIANT:
 * Reflection entries MUST be encrypted client-side in RAM BEFORE entering this queue.
 * Only ciphertexts ({ entry_date, ciphertext_base64, salt_base64, nonce_base64, mood_score, tags })
 * are ever stored in IndexedDB. Plaintext content is NEVER persisted here.
 */

import { ref } from 'vue'
import type { AxiosInstance } from 'axios'

export interface QueuedMutation {
  id: string
  url: string
  method: 'POST' | 'PUT' | 'DELETE' | 'PATCH'
  payload: any
  idempotencyKey: string
  createdAt: string
  retryCount: number
}

const DB_NAME = 'roubit_offline_db'
const DB_VERSION = 1
const STORE_NAME = 'mutation_queue'

export const pendingSyncCount = ref(0)
export const isSyncing = ref(false)

function getIndexedDB(): IDBFactory | null {
  if (typeof window !== 'undefined' && window.indexedDB) {
    return window.indexedDB
  }
  return null
}

export function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const idb = getIndexedDB()
    if (!idb) {
      return reject(new Error('IndexedDB is not supported in this environment'))
    }

    const request = idb.open(DB_NAME, DB_VERSION)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'id' })
      }
    }

    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

export async function enqueueMutation(
  item: Omit<QueuedMutation, 'id' | 'createdAt' | 'retryCount'>
): Promise<QueuedMutation> {
  const db = await openDb()
  const queued: QueuedMutation = {
    ...item,
    id: crypto.randomUUID(),
    createdAt: new Date().toISOString(),
    retryCount: 0,
  }

  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite')
    const store = tx.objectStore(STORE_NAME)
    const request = store.add(queued)

    request.onsuccess = () => {
      updatePendingCount()
      resolve(queued)
    }
    request.onerror = () => reject(request.error)
  })
}

export async function getQueuedMutations(): Promise<QueuedMutation[]> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readonly')
    const store = tx.objectStore(STORE_NAME)
    const request = store.getAll()

    request.onsuccess = () => {
      const items: QueuedMutation[] = request.result || []
      // Sort strictly FIFO by creation timestamp
      items.sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
      resolve(items)
    }
    request.onerror = () => reject(request.error)
  })
}

export async function removeMutation(id: string): Promise<void> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite')
    const store = tx.objectStore(STORE_NAME)
    const request = store.delete(id)

    request.onsuccess = () => {
      updatePendingCount()
      resolve()
    }
    request.onerror = () => reject(request.error)
  })
}

export async function updateMutationRetry(id: string, retryCount: number): Promise<void> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite')
    const store = tx.objectStore(STORE_NAME)
    const getReq = store.get(id)

    getReq.onsuccess = () => {
      const item: QueuedMutation = getReq.result
      if (item) {
        item.retryCount = retryCount
        store.put(item)
      }
      resolve()
    }
    getReq.onerror = () => reject(getReq.error)
  })
}

export async function clearQueue(): Promise<void> {
  const db = await openDb()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite')
    const store = tx.objectStore(STORE_NAME)
    const request = store.clear()

    request.onsuccess = () => {
      updatePendingCount()
      resolve()
    }
    request.onerror = () => reject(request.error)
  })
}

export async function updatePendingCount(): Promise<number> {
  try {
    const items = await getQueuedMutations()
    pendingSyncCount.value = items.length
    return items.length
  } catch {
    pendingSyncCount.value = 0
    return 0
  }
}

/**
 * Sequential FIFO replay engine.
 * Dispatches mutations with their original Idempotency-Key.
 */
export async function syncOfflineQueue(apiClientInstance: AxiosInstance): Promise<{ synced: number; failed: number }> {
  if (isSyncing.value) {
    return { synced: 0, failed: 0 }
  }

  isSyncing.value = true
  let synced = 0
  let failed = 0

  try {
    const items = await getQueuedMutations()
    for (const item of items) {
      if (typeof window !== 'undefined' && !window.navigator.onLine) {
        // Stop replay if connection dropped again
        break
      }

      try {
        await apiClientInstance.request({
          url: item.url,
          method: item.method,
          data: item.payload,
          headers: {
            'Idempotency-Key': item.idempotencyKey,
          },
          // Custom config parameter to avoid re-enqueuing during sync replay
          ...({ skipOfflineQueue: true } as any),
        })

        // Success (2xx or handled): remove from IndexedDB
        await removeMutation(item.id)
        synced++
      } catch (err: any) {
        const status = err.response?.status
        if (status === 409) {
          // 409 Conflict: Idempotent middleware already processed this exact request on server
          await removeMutation(item.id)
          synced++
        } else if (status >= 400 && status < 500) {
          // Permanent client validation error (400, 422): discard to avoid poisoning queue
          await removeMutation(item.id)
          failed++
        } else {
          // Network error or 5xx: increment retry and pause drain
          await updateMutationRetry(item.id, item.retryCount + 1)
          break
        }
      }
    }
  } finally {
    isSyncing.value = false
    await updatePendingCount()
  }

  return { synced, failed }
}

let syncInitialized = false

/**
 * Hooks browser connectivity and visibility listeners to automatically trigger replay.
 */
export function initOfflineSync(apiClientInstance: AxiosInstance): void {
  if (syncInitialized || typeof window === 'undefined') return
  syncInitialized = true

  // Initial count update
  updatePendingCount()

  window.addEventListener('online', () => {
    syncOfflineQueue(apiClientInstance)
  })

  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible' && window.navigator.onLine) {
      syncOfflineQueue(apiClientInstance)
    }
  })
}
