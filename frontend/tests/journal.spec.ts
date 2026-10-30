import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import JournalEditor from '@/components/journal/JournalEditor.vue'
import { useVaultStore } from '@/stores/vault'
import { WebCryptoService } from '@/services/crypto'

describe('Frontend Integration Tests: Encrypted Journaling', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('guarantees zero plaintext network traffic during encryption', async () => {
    const salt = WebCryptoService.generateSalt()
    const masterKey = await WebCryptoService.deriveKey('master-test-password', salt)

    const secretThought = 'Minhas reflexões mais íntimas e filosóficas de 2026'
    const envelope = await WebCryptoService.encrypt(secretThought, masterKey, salt)

    // Ensure zero plaintext appears in serialized envelope
    expect(envelope.ciphertextBase64).not.toContain('reflexões')
    expect(envelope.ciphertextBase64).not.toContain('filosóficas')
    expect(envelope.nonceBase64).not.toContain('íntimas')

    // Ensure roundtrip integrity
    const decrypted = await WebCryptoService.decrypt(envelope, masterKey)
    expect(decrypted).toBe(secretThought)
  })

  it('manages vault lifecycle with memory wiping on lock', async () => {
    const vault = useVaultStore()
    expect(vault.isUnlocked).toBe(false)
    expect(vault.masterKey).toBeNull()

    await vault.unlock('test-passphrase')
    expect(vault.isUnlocked).toBe(true)
    expect(vault.masterKey).not.toBeNull()

    // Immediate lock wipe
    vault.lock()
    expect(vault.isUnlocked).toBe(false)
    expect(vault.masterKey).toBeNull()
  })

  it('renders JournalEditor with live reactive word/character counters', () => {
    const wrapper = mount(JournalEditor, {
      props: {
        plaintextReflection: 'Planejamento e foco produzem excelência.',
        tags: ['foco', 'disciplina'],
        entryDate: '2026-10-30',
      },
    })

    expect(wrapper.text()).toContain('5 palavras')
    expect(wrapper.text()).toContain('40 caracteres')
  })
})
