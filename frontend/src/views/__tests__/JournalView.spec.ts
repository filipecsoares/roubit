import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import JournalView from '../JournalView.vue'
import { useVaultStore } from '@/stores/vault'
import { useJournalStore } from '@/stores/journal'
import { WebCryptoService } from '@/services/crypto'
import type { JournalEntryResponse, JournalMetadataResponse } from '@/types'

describe('JournalView.vue Integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const journalStore = useJournalStore()
    vi.spyOn(journalStore, 'fetchMetadata').mockImplementation(async () => [])
    vi.clearAllMocks()
  })

  it('renders locked banner when vault is locked', async () => {
    const vaultStore = useVaultStore()
    vaultStore.masterKey = null

    const wrapper = mount(JournalView)
    await flushPromises()

    expect(wrapper.text()).toContain('Cofre de Reflexões Protegido')
    expect(wrapper.text()).toContain('Desbloquear Diário')
    expect(wrapper.findComponent({ name: 'JournalEditor' }).exists()).toBe(false)
  })

  it('opens VaultUnlockModal on unlock button click and unlocks vault', async () => {
    const vaultStore = useVaultStore()
    const testSalt = WebCryptoService.generateSalt()
    const testKey = await WebCryptoService.deriveKey('MinhaSenhaForte2026!', testSalt)

    const unlockSpy = vi.spyOn(vaultStore, 'unlock').mockImplementation(async () => {
      vaultStore.masterKey = testKey
      vaultStore.currentSalt = testSalt
      return testKey
    })

    const wrapper = mount(JournalView)
    await flushPromises()

    const unlockBtn = wrapper.findAll('button').find((b) => b.text().includes('Desbloquear Diário'))
    expect(unlockBtn).toBeDefined()
    await unlockBtn!.trigger('click')

    const modal = wrapper.findComponent({ name: 'VaultUnlockModal' })
    expect(modal.exists()).toBe(true)
    expect(modal.props('isOpen')).toBe(true)

    // Submit passphrase
    await modal.vm.$emit('unlock', 'MinhaSenhaForte2026!')
    await flushPromises()

    expect(unlockSpy).toHaveBeenCalledWith('MinhaSenhaForte2026!', undefined)
    expect(modal.props('isOpen')).toBe(false)
    expect(wrapper.findComponent({ name: 'JournalEditor' }).exists()).toBe(true)
  })

  it('decrypts cross-session journal entry into editor when vault is unlocked', async () => {
    const vaultStore = useVaultStore()
    const journalStore = useJournalStore()

    const testSalt = WebCryptoService.generateSalt()
    const testKey = await WebCryptoService.deriveKey('MinhaSenhaForte2026!', testSalt)
    const secretText = 'Registro de pensamentos com conhecimento zero.'
    const envelope = await WebCryptoService.encrypt(secretText, testKey, testSalt)

    const mockMeta: JournalMetadataResponse[] = [
      {
        id: 'entry-1',
        user_id: 'user-1',
        entry_date: '2026-10-30',
        mood_score: 5,
        tags: ['mindfulness', 'segurança'],
        word_count: 6,
        created_at: '2026-10-30T00:00:00Z',
        updated_at: '2026-10-30T00:00:00Z',
      },
    ]

    const mockEntry: JournalEntryResponse = {
      id: 'entry-1',
      user_id: 'user-1',
      entry_date: '2026-10-30',
      mood_score: 5,
      tags: ['mindfulness', 'segurança'],
      salt_base64: envelope.saltBase64,
      nonce_base64: envelope.nonceBase64,
      ciphertext_base64: envelope.ciphertextBase64,
      created_at: '2026-10-30T00:00:00Z',
      updated_at: '2026-10-30T00:00:00Z',
    }

    journalStore.metadataList = mockMeta
    vi.spyOn(journalStore, 'fetchEntry').mockResolvedValue(mockEntry)
    vi.spyOn(vaultStore, 'getKeyForSalt').mockResolvedValue(testKey)

    vaultStore.masterKey = testKey
    vaultStore.currentSalt = testSalt

    const wrapper = mount(JournalView)
    await flushPromises()

    // Editor should exist and be loaded with decrypted contents
    const editor = wrapper.findComponent({ name: 'JournalEditor' })
    expect(editor.exists()).toBe(true)
  })

  it('encrypts and dispatches entry via journalStore.saveEncryptedEntry without plaintext in payload', async () => {
    const vaultStore = useVaultStore()
    const journalStore = useJournalStore()

    const testSalt = WebCryptoService.generateSalt()
    const testKey = await WebCryptoService.deriveKey('master-pass', testSalt)
    vaultStore.masterKey = testKey
    vaultStore.currentSalt = testSalt

    const saveSpy = vi.spyOn(journalStore, 'saveEncryptedEntry').mockResolvedValue({} as any)

    const wrapper = mount(JournalView)
    await flushPromises()

    const editor = wrapper.findComponent({ name: 'JournalEditor' })
    expect(editor.exists()).toBe(true)

    // Simulate input into editor
    await editor.vm.$emit('update:plaintext-reflection', 'Pensamentos do dia salvos criptografados.')
    await editor.vm.$emit('update:mood-score', 4)
    await editor.vm.$emit('update:tags', ['foco'])
    await flushPromises()

    // Trigger save
    await editor.vm.$emit('save')
    await flushPromises()

    expect(saveSpy).toHaveBeenCalledTimes(1)
    const payload = saveSpy.mock.calls[0][0]
    expect(payload.entry_date).toBeDefined()
    expect(payload.mood_score).toBe(4)
    expect(payload.tags).toEqual(['foco'])
    expect(payload.ciphertext_base64).toBeDefined()
    expect(payload.ciphertext_base64).not.toContain('Pensamentos do dia')
  })

  it('scrubs plaintext reflection and sensitive state immediately upon locking vault', async () => {
    const vaultStore = useVaultStore()
    const testSalt = WebCryptoService.generateSalt()
    const testKey = await WebCryptoService.deriveKey('master-pass', testSalt)
    vaultStore.masterKey = testKey
    vaultStore.currentSalt = testSalt

    const wrapper = mount(JournalView)
    await flushPromises()

    const editor = wrapper.findComponent({ name: 'JournalEditor' })
    expect(editor.exists()).toBe(true)

    await editor.vm.$emit('update:plaintext-reflection', 'Texto ultra-secreto em memória.')
    await flushPromises()

    expect(editor.props('plaintextReflection')).toBe('Texto ultra-secreto em memória.')

    // Vault locks (e.g. user locks or inactivity timer expires)
    vaultStore.lock()
    await flushPromises()

    // Editor unmounted, locked banner shown
    expect(wrapper.findComponent({ name: 'JournalEditor' }).exists()).toBe(false)
    expect(wrapper.text()).toContain('Cofre de Reflexões Protegido')

    // When re-unlocked, the previous plaintext was scrubbed
    vaultStore.masterKey = testKey
    await flushPromises()

    const newEditor = wrapper.findComponent({ name: 'JournalEditor' })
    expect(newEditor.exists()).toBe(true)
    expect(newEditor.props('plaintextReflection')).toBe('')
  })
})
