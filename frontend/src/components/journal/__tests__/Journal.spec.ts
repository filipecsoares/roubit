import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import LockStatusBadge from '../LockStatusBadge.vue'
import MoodSelector from '../MoodSelector.vue'
import TagInput from '../TagInput.vue'
import VaultUnlockModal from '../VaultUnlockModal.vue'
import JournalEditor from '../JournalEditor.vue'
import { useVaultStore } from '@/stores/vault'
import { WebCryptoService } from '@/services/crypto'

describe('Journal Components & Security Guarantees', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    document.body.innerHTML = ''
  })

  describe('LockStatusBadge.vue', () => {
    it('renders locked state and emits unlock', async () => {
      const wrapper = mount(LockStatusBadge, {
        props: { isUnlocked: false },
      })

      expect(wrapper.text()).toContain('Cofre Bloqueado')
      expect(wrapper.text()).toContain('Desbloquear')

      const unlockBtn = wrapper.find('button')
      await unlockBtn.trigger('click')
      expect(wrapper.emitted('unlock')).toBeTruthy()
    })

    it('renders unlocked state and emits lock', async () => {
      const wrapper = mount(LockStatusBadge, {
        props: { isUnlocked: true },
      })

      expect(wrapper.text()).toContain('Cofre Desbloqueado (Chave na RAM)')
      expect(wrapper.text()).toContain('Bloquear')

      const lockBtn = wrapper.find('button')
      await lockBtn.trigger('click')
      expect(wrapper.emitted('lock')).toBeTruthy()
    })
  })

  describe('MoodSelector.vue', () => {
    it('renders all 5 mood options and emits value on select', async () => {
      const wrapper = mount(MoodSelector, {
        props: { modelValue: 3 },
      })

      const buttons = wrapper.findAll('button')
      expect(buttons.length).toBe(5)

      // Click on 5th button (score 5)
      await buttons[4].trigger('click')
      expect(wrapper.emitted('update:modelValue')).toBeTruthy()
      expect(wrapper.emitted('update:modelValue')![0]).toEqual([5])
    })
  })

  describe('TagInput.vue', () => {
    it('displays existing tags and removes a tag on click', async () => {
      const wrapper = mount(TagInput, {
        props: { modelValue: ['foco', 'estudos'] },
      })

      expect(wrapper.text()).toContain('foco')
      expect(wrapper.text()).toContain('estudos')

      // Click remove on first tag
      const removeButtons = wrapper.findAll('button')
      await removeButtons[0].trigger('click')

      expect(wrapper.emitted('update:modelValue')).toBeTruthy()
      expect(wrapper.emitted('update:modelValue')![0]).toEqual([['estudos']])
    })
  })

  describe('VaultUnlockModal.vue', () => {
    it('emits unlock event with entered passphrase', async () => {
      const wrapper = mount(VaultUnlockModal, {
        props: { isOpen: true },
        attachTo: document.body,
      })

      const input = document.body.querySelector('input[placeholder="Digite sua senha mestre..."]') as HTMLInputElement
      expect(input).not.toBeNull()
      input.value = 'super-secret-passphrase'
      input.dispatchEvent(new Event('input'))
      await wrapper.vm.$nextTick()

      const submitBtn = document.body.querySelector('button.bg-brand-600') as HTMLButtonElement
      submitBtn.click()
      await wrapper.vm.$nextTick()

      expect(wrapper.emitted('unlock')).toBeTruthy()
      expect(wrapper.emitted('unlock')![0]).toEqual(['super-secret-passphrase'])
    })
  })

  describe('JournalEditor.vue', () => {
    it('computes word count and character count', () => {
      const wrapper = mount(JournalEditor, {
        props: {
          plaintextReflection: 'Hoje aprendi sobre Rust e WebAssembly no navegador.',
          tags: ['rust', 'web'],
          entryDate: '2026-10-30',
        },
      })

      expect(wrapper.text()).toContain('8 palavras')
      expect(wrapper.text()).toContain('51 caracteres')
    })

    it('emits save when clicking save button', async () => {
      const wrapper = mount(JournalEditor, {
        props: {
          plaintextReflection: 'Minhas reflexões secretas',
          tags: [],
          entryDate: '2026-10-30',
        },
      })

      const saveBtn = wrapper.find('button.bg-brand-600')
      await saveBtn.trigger('click')

      expect(wrapper.emitted('save')).toBeTruthy()
    })
  })

  describe('Zero-Knowledge & Memory Safety Policy', () => {
    it('ensures ciphertext payload contains ZERO plaintext leakage', async () => {
      const salt = WebCryptoService.generateSalt()
      const key = await WebCryptoService.deriveKey('vault-passphrase-2026', salt)

      const sensitivePlaintext = 'Este é um segredo confidencial ultra secreto'
      const envelope = await WebCryptoService.encrypt(sensitivePlaintext, key, salt)

      // Verify that none of the words from sensitivePlaintext exist in ciphertextBase64, salt, or nonce
      expect(envelope.ciphertextBase64).not.toContain('segredo')
      expect(envelope.ciphertextBase64).not.toContain('confidencial')
      expect(envelope.ciphertextBase64).not.toContain('secreto')
      expect(envelope.nonceBase64).not.toContain('segredo')
      expect(envelope.saltBase64).not.toContain('segredo')

      // Verify exact decryption
      const decrypted = await WebCryptoService.decrypt(envelope, key)
      expect(decrypted).toBe(sensitivePlaintext)
    })

    it('wipes CryptoKey from volatile memory when vault is locked', async () => {
      const vaultStore = useVaultStore()
      expect(vaultStore.isUnlocked).toBe(false)
      expect(vaultStore.masterKey).toBeNull()

      await vaultStore.unlock('temporary-passphrase')
      expect(vaultStore.isUnlocked).toBe(true)
      expect(vaultStore.masterKey).not.toBeNull()

      // Lock vault
      vaultStore.lock()
      expect(vaultStore.isUnlocked).toBe(false)
      expect(vaultStore.masterKey).toBeNull()
      expect(vaultStore.currentSalt).toBeNull()
    })

    it('decrypts entries across sessions using pre-existing salt_base64 via getKeyForSalt', async () => {
      const vaultStore = useVaultStore()
      const passphrase = 'cross-session-passphrase-2026'

      // Session 1: Unlock vault and encrypt reflection
      await vaultStore.unlock(passphrase)
      const session1Salt = vaultStore.currentSalt!
      const envelope = await WebCryptoService.encrypt(
        'Reflexão salva na sessão 1 que deve ser decifrada na sessão 2',
        vaultStore.masterKey!,
        session1Salt
      )

      // End of Session 1: Lock vault (wipes masterKey and currentSalt)
      vaultStore.lock()
      expect(vaultStore.isUnlocked).toBe(false)

      // Session 2: Unlock vault (without existing salt, generating a different random salt initially)
      await vaultStore.unlock(passphrase)
      expect(vaultStore.isUnlocked).toBe(true)

      // Derive/retrieve key for Session 1's entry salt_base64
      const entryKey1 = await vaultStore.getKeyForSalt(envelope.saltBase64)
      const decrypted = await WebCryptoService.decrypt(envelope, entryKey1)
      expect(decrypted).toBe('Reflexão salva na sessão 1 que deve ser decifrada na sessão 2')

      // Subsequent lookup for same saltBase64 hits O(1) in-memory Map cache
      const entryKey2 = await vaultStore.getKeyForSalt(envelope.saltBase64)
      expect(entryKey2).toBe(entryKey1)
    })

    it('auto-locks after 15 minutes of inactivity and resets timer on activity', async () => {
      const { vi } = await import('vitest')
      vi.useFakeTimers()
      try {
        const vaultStore = useVaultStore()
        await vaultStore.unlock('timer-test-passphrase')
        expect(vaultStore.isUnlocked).toBe(true)

        // Advance 10 minutes and simulate user activity
        vi.advanceTimersByTime(10 * 60 * 1000)
        expect(vaultStore.isUnlocked).toBe(true)
        vaultStore.resetInactivityTimer()

        // Advance another 10 minutes (total 20m, but only 10m since last activity) -> still unlocked
        vi.advanceTimersByTime(10 * 60 * 1000)
        expect(vaultStore.isUnlocked).toBe(true)

        // Advance remaining 5 minutes of inactivity -> auto-locked
        vi.advanceTimersByTime(5 * 60 * 1000)
        expect(vaultStore.isUnlocked).toBe(false)
        expect(vaultStore.masterKey).toBeNull()
        expect(vaultStore.currentSalt).toBeNull()
      } finally {
        vi.useRealTimers()
      }
    })
  })
})
