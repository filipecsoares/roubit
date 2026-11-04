import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { WebCryptoService, base64ToUint8Array, uint8ArrayToBase64 } from '@/services/crypto'

const AUTO_LOCK_TIMEOUT_MS = 15 * 60 * 1000 // 15 minutes of inactivity

export const useVaultStore = defineStore('vault', () => {
  const masterKey = ref<CryptoKey | null>(null)
  const currentSalt = ref<Uint8Array | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  let autoLockTimer: ReturnType<typeof setTimeout> | null = null
  let activePassphrase: string | null = null
  const derivedKeysBySalt = new Map<string, CryptoKey>()

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
      const saltBase64 = existingSaltBase64 || uint8ArrayToBase64(salt)

      const key = await WebCryptoService.deriveKey(passphrase, salt)
      activePassphrase = passphrase
      derivedKeysBySalt.set(saltBase64, key)
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

  async function getKeyForSalt(saltBase64: string): Promise<CryptoKey> {
    const cached = derivedKeysBySalt.get(saltBase64)
    if (cached) {
      masterKey.value = cached
      currentSalt.value = base64ToUint8Array(saltBase64)
      resetInactivityTimer()
      return cached
    }

    if (!activePassphrase) {
      throw new Error('Cofre bloqueado: senha mestre ausente na memória')
    }

    loading.value = true
    error.value = null
    try {
      const salt = base64ToUint8Array(saltBase64)
      const key = await WebCryptoService.deriveKey(activePassphrase, salt)
      derivedKeysBySalt.set(saltBase64, key)
      masterKey.value = key
      currentSalt.value = salt
      resetInactivityTimer()
      return key
    } catch (err: any) {
      error.value = err.message || 'Falha ao derivar chave para o salt da entrada'
      throw err
    } finally {
      loading.value = false
    }
  }

  function lock(): void {
    activePassphrase = null
    derivedKeysBySalt.clear()
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
    getKeyForSalt,
    lock,
    resetInactivityTimer,
  }
})
