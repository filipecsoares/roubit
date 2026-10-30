<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  Lock,
  ChevronLeft,
  ChevronRight,
  Calendar,
  AlertCircle,
  Sparkles,
  KeyRound,
  Trash2,
} from 'lucide-vue-next'
import { useVaultStore } from '@/stores/vault'
import { useJournalStore } from '@/stores/journal'
import { WebCryptoService } from '@/services/crypto'
import LockStatusBadge from '@/components/journal/LockStatusBadge.vue'
import VaultUnlockModal from '@/components/journal/VaultUnlockModal.vue'
import JournalEditor from '@/components/journal/JournalEditor.vue'

const vaultStore = useVaultStore()
const journalStore = useJournalStore()

const todayStr = new Date().toISOString().split('T')[0]
const entryDate = ref(todayStr)
const plaintextReflection = ref('')
const moodScore = ref(3)
const tags = ref<string[]>([])

const isUnlockModalOpen = ref(false)
const unlockError = ref<string | null>(null)
const isSaving = ref(false)
const toastMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

function showToast(text: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = { text, type }
  setTimeout(() => {
    if (toastMessage.value?.text === text) {
      toastMessage.value = null
    }
  }, 4000)
}

const formattedDateDisplay = computed(() => {
  const parts = entryDate.value.split('-').map(Number)
  const dateObj = new Date(parts[0], parts[1] - 1, parts[2])
  return dateObj.toLocaleDateString('pt-BR', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
})

function changeDate(daysOffset: number) {
  const parts = entryDate.value.split('-').map(Number)
  const d = new Date(parts[0], parts[1] - 1, parts[2])
  d.setDate(d.getDate() + daysOffset)
  entryDate.value = d.toISOString().split('T')[0]
  loadEntryForDate()
}

function setToday() {
  entryDate.value = todayStr
  loadEntryForDate()
}

const hasEntryForCurrentDate = computed(() => {
  return journalStore.metadataList.some((m) => m.entry_date === entryDate.value)
})

async function handleDeleteEntry() {
  const meta = journalStore.metadataList.find((m) => m.entry_date === entryDate.value)
  if (!meta) return
  if (!confirm(`Tem certeza que deseja excluir a reflexão do dia ${entryDate.value}?`)) return
  try {
    await journalStore.deleteEntry(meta.id)
    plaintextReflection.value = ''
    moodScore.value = 3
    tags.value = []
    showToast(`Entrada do dia ${entryDate.value} excluída com sucesso.`)
  } catch (err: any) {
    showToast(err.message || 'Erro ao excluir reflexão', 'error')
  }
}

async function loadEntryForDate() {
  if (!vaultStore.isUnlocked) return

  // Find metadata for entryDate
  const meta = journalStore.metadataList.find((m) => m.entry_date === entryDate.value)
  if (!meta) {
    plaintextReflection.value = ''
    moodScore.value = 3
    tags.value = []
    return
  }

  try {
    const entry = await journalStore.fetchEntry(meta.id)
    moodScore.value = entry.mood_score || 3
    tags.value = entry.tags || []

    if (vaultStore.masterKey) {
      const decrypted = await WebCryptoService.decrypt(
        {
          saltBase64: entry.salt_base64,
          nonceBase64: entry.nonce_base64,
          ciphertextBase64: entry.ciphertext_base64,
        },
        vaultStore.masterKey
      )
      plaintextReflection.value = decrypted
    }
  } catch (err: any) {
    showToast('Falha ao decifrar reflexão para esta data', 'error')
  }
}

async function handleUnlock(passphrase: string) {
  unlockError.value = null
  try {
    await vaultStore.unlock(passphrase)
    isUnlockModalOpen.value = false
    showToast('Diário desbloqueado com sucesso! Chave retida estritamente em RAM.')
    await journalStore.fetchMetadata()
    await loadEntryForDate()
  } catch (err: any) {
    unlockError.value = err.message || 'Senha mestre incorreta ou erro de derivação'
  }
}

function handleLock() {
  vaultStore.lock()
  // Securely wipe plaintext from component state immediately
  plaintextReflection.value = ''
  moodScore.value = 3
  tags.value = []
  showToast('Cofre bloqueado. Chaves e textos limpos da memória.')
}

async function handleSave() {
  if (!vaultStore.masterKey) {
    isUnlockModalOpen.value = true
    return
  }

  if (!plaintextReflection.value.trim()) return

  isSaving.value = true
  try {
    // 1. Client-side encryption via AES-256-GCM
    const salt = vaultStore.currentSalt || WebCryptoService.generateSalt()
    const envelope = await WebCryptoService.encrypt(
      plaintextReflection.value,
      vaultStore.masterKey,
      salt
    )

    // 2. Dispatch ciphertexts envelope to server (no plaintext transmitted!)
    await journalStore.saveEncryptedEntry({
      entry_date: entryDate.value,
      mood_score: moodScore.value,
      tags: tags.value,
      salt_base64: envelope.saltBase64,
      nonce_base64: envelope.nonceBase64,
      ciphertext_base64: envelope.ciphertextBase64,
    })

    showToast('Reflexão cifrada e salva com sucesso no cofre!')
  } catch (err: any) {
    showToast(err.message || 'Erro ao criptografar ou salvar reflexão', 'error')
  } finally {
    isSaving.value = false
  }
}

onMounted(async () => {
  try {
    await journalStore.fetchMetadata()
  } catch (e) {}
})
</script>

<template>
  <div class="max-w-4xl mx-auto px-4 py-8 space-y-8 animate-fade-in">
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
      class="fixed bottom-6 right-6 z-50 flex items-center gap-2 px-4 py-3 rounded-xl border shadow-xl text-sm transition-all"
      :class="
        toastMessage.type === 'success'
          ? 'bg-emerald-950/90 border-emerald-500/50 text-emerald-200'
          : 'bg-rose-950/90 border-rose-500/50 text-rose-200'
      "
    >
      <AlertCircle v-if="toastMessage.type === 'error'" class="w-4 h-4 shrink-0" />
      <Sparkles v-else class="w-4 h-4 shrink-0 text-emerald-400" />
      <span>{{ toastMessage.text }}</span>
    </div>

    <!-- Header & Security Status -->
    <header class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-3xl font-extrabold tracking-tight text-white font-outfit">
          Diário Criptografado
        </h1>
        <p class="text-sm text-slate-400 mt-1">
          Escrita reflexiva com garantia de Conhecimento Zero (Zero-Knowledge): cifragem local com AES-256-GCM e Argon2id.
        </p>
      </div>

      <LockStatusBadge
        :is-unlocked="vaultStore.isUnlocked"
        :loading="vaultStore.loading"
        @unlock="isUnlockModalOpen = true"
        @lock="handleLock"
      />
    </header>

    <!-- Locked Vault Banner (When Vault is Locked) -->
    <section
      v-if="!vaultStore.isUnlocked"
      class="p-10 rounded-2xl bg-surface-card border border-surface-border text-center space-y-5 shadow-lg"
    >
      <div class="w-14 h-14 mx-auto rounded-2xl bg-brand-500/10 flex items-center justify-center border border-brand-500/20 text-brand-400 shadow-inner">
        <Lock class="w-7 h-7" />
      </div>
      <div class="max-w-md mx-auto space-y-2">
        <h2 class="text-xl font-bold text-white">Cofre de Reflexões Protegido</h2>
        <p class="text-xs text-slate-400 leading-relaxed">
          Suas anotações são armazenadas cifradas no servidor. Para escrever ou ler reflexões, desbloqueie seu cofre fornecendo a senha mestre para derivação da chave em memória.
        </p>
      </div>
      <div>
        <button
          type="button"
          @click="isUnlockModalOpen = true"
          class="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-brand-600 hover:bg-brand-500 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all"
        >
          <KeyRound class="w-4 h-4" />
          Desbloquear Diário
        </button>
      </div>
    </section>

    <!-- Unlocked Editor Workspace (When Vault is Unlocked) -->
    <section v-else class="space-y-6">
      <!-- Date Navigation Toolbar -->
      <div class="flex items-center justify-between p-3 rounded-xl bg-surface-card border border-surface-border">
        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="changeDate(-1)"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
            title="Dia anterior"
          >
            <ChevronLeft class="w-4 h-4" />
          </button>
          <span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-lg bg-surface-dark border border-surface-border text-xs font-semibold text-white">
            <Calendar class="w-3.5 h-3.5 text-brand-400" />
            <span class="capitalize">{{ formattedDateDisplay }}</span>
          </span>
          <button
            type="button"
            @click="changeDate(1)"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
            title="Próximo dia"
          >
            <ChevronRight class="w-4 h-4" />
          </button>
        </div>

        <div class="flex items-center gap-2">
          <button
            v-if="hasEntryForCurrentDate"
            type="button"
            @click="handleDeleteEntry"
            title="Excluir reflexão deste dia"
            class="p-1.5 rounded-lg text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
          >
            <Trash2 class="w-4 h-4" />
          </button>

          <button
            type="button"
            @click="setToday"
            class="px-3 py-1 rounded-lg text-xs font-medium text-slate-400 hover:text-white hover:bg-surface-border/40 transition-colors"
          >
            Hoje
          </button>
        </div>
      </div>

      <!-- Journal Editor -->
      <JournalEditor
        :entry-date="entryDate"
        :plaintext-reflection="plaintextReflection"
        :mood-score="moodScore"
        :tags="tags"
        :is-saving="isSaving"
        @update:plaintext-reflection="(val) => (plaintextReflection = val)"
        @update:mood-score="(val) => (moodScore = val)"
        @update:tags="(val) => (tags = val)"
        @save="handleSave"
      />
    </section>

    <!-- Unlock Passphrase Modal -->
    <VaultUnlockModal
      :is-open="isUnlockModalOpen"
      :loading="vaultStore.loading"
      :error-message="unlockError"
      @close="isUnlockModalOpen = false"
      @unlock="handleUnlock"
    />
  </div>
</template>
