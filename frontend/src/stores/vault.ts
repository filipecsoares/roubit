import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { WebCryptoService, base64ToUint8Array } from '@/services/crypto'

const AUTO_LOCK_TIMEOUT_MS = 15 * 60 * 1000 // 15 minutes of inactivity

export const useVaultStore = defineStore('vault', () => {
  const masterKey = ref<CryptoKey | null>(null)
  const currentSalt = ref<Uint8Array | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  let autoLockTimer: ReturnType<typeof setTimeout> | null = null

  const isUnlocked = computed(() => !!masterKey.value)

  function resetInactivityTimer(): void {
    if (autoLockTimer) {
      clearTimeout(autoLockTimer)
    }
    if (masterKey.value) {
      autoLockTimer = setTimeout(() => {
        lock()
      }, AUTO_LOCK_TIMEOUT_MS)
    }
  }

  async function unlock(passphrase: string, existingSaltBase64?: string): Promise<CryptoKey> {
    loading.value = true
    error.value = null
    try {
      const salt = existingSaltBase64
        ? base64ToUint8Array(existingSaltBase64)
        : WebCryptoService.generateSalt()

      const key = await WebCryptoService.deriveKey(passphrase, salt)
      masterKey.value = key
      currentSalt.value = salt
      resetInactivityTimer()
      return key
    } catch (err: any) {
      error.value = err.message || 'Falha ao derivar chave do diário'
      throw err
    } finally {
      loading.value = false
    }
  }

  function lock(): void {
    masterKey.value = null
    currentSalt.value = null
    if (autoLockTimer) {
      clearTimeout(autoLockTimer)
      autoLockTimer = null
    }
  }

  return {
    masterKey,
    currentSalt,
    isUnlocked,
    loading,
    error,
    unlock,
    lock,
    resetInactivityTimer,
  }
})
