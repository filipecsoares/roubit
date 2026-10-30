import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { WebCryptoService, uint8ArrayToBase64, base64ToUint8Array } from '../crypto'
import { useVaultStore } from '@/stores/vault'

describe('WebCryptoService (AES-256-GCM & Argon2id WASM)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('generates a 16-byte cryptographically secure salt', () => {
    const salt = WebCryptoService.generateSalt()
    expect(salt).toBeInstanceOf(Uint8Array)
    expect(salt.length).toBe(16)
  })

  it('correctly converts between Uint8Array and Base64', () => {
    const bytes = new Uint8Array([0, 1, 2, 253, 254, 255])
    const base64 = uint8ArrayToBase64(bytes)
    const decoded = base64ToUint8Array(base64)
    expect(decoded).toEqual(bytes)
  })

  it('derives a non-extractable AES-GCM CryptoKey from passphrase and salt', async () => {
    const salt = WebCryptoService.generateSalt()
    const key = await WebCryptoService.deriveKey('super-secret-passphrase', salt)

    expect(key).toBeDefined()
    expect(key.type).toBe('secret')
    expect(key.extractable).toBe(false)
    expect(key.algorithm.name).toBe('AES-GCM')
  })

  it('encrypts and decrypts journal reflection restoring exact plaintext', async () => {
    const salt = WebCryptoService.generateSalt()
    const key = await WebCryptoService.deriveKey('my-vault-passphrase-123', salt)

    const originalText = 'Reflexão noturna: o dia foi extremamente produtivo e com foco profundo em Rust e Vue 3.'
    const envelope = await WebCryptoService.encrypt(originalText, key, salt)

    expect(envelope.saltBase64).toBeDefined()
    expect(envelope.nonceBase64).toBeDefined()
    expect(envelope.ciphertextBase64).toBeDefined()

    const decrypted = await WebCryptoService.decrypt(envelope, key)
    expect(decrypted).toBe(originalText)
  })

  it('rejects tampered ciphertext with decryption authentication error', async () => {
    const salt = WebCryptoService.generateSalt()
    const key = await WebCryptoService.deriveKey('tamper-test-passphrase', salt)

    const envelope = await WebCryptoService.encrypt('Confidential journal note', key, salt)

    // Flip bits in the ciphertext Base64
    const rawCiphertext = base64ToUint8Array(envelope.ciphertextBase64)
    rawCiphertext[0] ^= 0xFF
    envelope.ciphertextBase64 = uint8ArrayToBase64(rawCiphertext)

    await expect(WebCryptoService.decrypt(envelope, key)).rejects.toThrow()
  })

  it('rejects decryption when using a key derived from a wrong passphrase', async () => {
    const salt = WebCryptoService.generateSalt()
    const keyCorrect = await WebCryptoService.deriveKey('correct-passphrase', salt)
    const keyWrong = await WebCryptoService.deriveKey('wrong-passphrase', salt)

    const envelope = await WebCryptoService.encrypt('Top secret reflection', keyCorrect, salt)

    await expect(WebCryptoService.decrypt(envelope, keyWrong)).rejects.toThrow()
  })

  describe('useVaultStore', () => {
    it('manages volatile unlock and lock lifecycle in memory', async () => {
      const vaultStore = useVaultStore()
      expect(vaultStore.isUnlocked).toBe(false)
      expect(vaultStore.masterKey).toBeNull()

      await vaultStore.unlock('vault-password')
      expect(vaultStore.isUnlocked).toBe(true)
      expect(vaultStore.masterKey).not.toBeNull()

      vaultStore.lock()
      expect(vaultStore.isUnlocked).toBe(false)
      expect(vaultStore.masterKey).toBeNull()
    })
  })
})
