<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
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
import { formatLocalDate, addDaysLocal, parseLocalDate } from '@/utils/date'
import { useToast, extractErrorMessage } from '@/composables/useToast'

const vaultStore = useVaultStore()
const journalStore = useJournalStore()
const { toastMessage, showToast } = useToast()

const todayStr = formatLocalDate()
const entryDate = ref(todayStr)
const plaintextReflection = ref('')
const moodScore = ref(3)
const tags = ref<string[]>([])

const isUnlockModalOpen = ref(false)
const unlockError = ref<string | null>(null)
const isSaving = ref(false)

// Scrub plaintext and metadata state whenever vault locks (manual or inactivity timeout)
watch(
  () => vaultStore.isUnlocked,
  (unlocked) => {
    if (!unlocked) {
      plaintextReflection.value = ''
      moodScore.value = 3
      tags.value = []
      journalStore.currentEntry = null
    }
  }
)

const formattedDateDisplay = computed(() => {
  const dateObj = parseLocalDate(entryDate.value)
  return dateObj.toLocaleDateString('pt-BR', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
})

function changeDate(daysOffset: number) {
  entryDate.value = addDaysLocal(entryDate.value, daysOffset)
  vaultStore.resetInactivityTimer()
  loadEntryForDate()
}

function setToday() {
  entryDate.value = todayStr
  vaultStore.resetInactivityTimer()
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
    vaultStore.resetInactivityTimer()
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
  vaultStore.resetInactivityTimer()

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

    const entryKey = await vaultStore.getKeyForSalt(entry.salt_base64)
    const decrypted = await WebCryptoService.decrypt(
      {
        saltBase64: entry.salt_base64,
        nonceBase64: entry.nonce_base64,
        ciphertextBase64: entry.ciphertext_base64,
      },
      entryKey
    )
    plaintextReflection.value = decrypted
  } catch (err: any) {
    showToast('Falha ao decifrar reflexão para esta data', 'error')
  }
}

async function handleUnlock(passphrase: string) {
  unlockError.value = null
  try {
    if (journalStore.metadataList.length === 0) {
      try {
        await journalStore.fetchMetadata()
      } catch {
        // Ignore network error during initial metadata check
      }
    }

    const targetMeta =
      journalStore.metadataList.find((m) => m.entry_date === entryDate.value) ||
      journalStore.metadataList[0]
    let existingSaltBase64: string | undefined
    if (targetMeta) {
      try {
        const existingEntry = await journalStore.fetchEntry(targetMeta.id)
        existingSaltBase64 = existingEntry.salt_base64
      } catch {
        // Fallback to new salt if entry fetch fails
      }
    }

    await vaultStore.unlock(passphrase, existingSaltBase64)
    isUnlockModalOpen.value = false
    showToast('Diário desbloqueado com sucesso! Chave retida estritamente em RAM.')
    await journalStore.fetchMetadata()
    await loadEntryForDate()
  } catch (err: any) {
    unlockError.value = extractErrorMessage(err, 'Senha mestre incorreta ou erro de derivação')
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
    vaultStore.resetInactivityTimer()
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
    showToast(err, 'error')
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
  <div class="space-y-6 sm:space-y-8 animate-fade-in max-w-4xl mx-auto">
    <!-- Toast Notification -->
    <div
      v-if="toastMessage"
      role="status"
      aria-live="polite"
      class="fixed bottom-20 md:bottom-6 right-4 sm:right-6 z-50 flex items-center gap-2 px-4 py-3 rounded-xl border shadow-xl text-sm transition-all"
      :class="
        toastMessage.type === 'success'
          ? 'bg-emerald-950/95 border-emerald-500/50 text-emerald-200'
          : 'bg-rose-950/95 border-rose-500/50 text-rose-200'
      "
    >
      <AlertCircle v-if="toastMessage.type === 'error'" class="w-4 h-4 shrink-0" />
      <Sparkles v-else class="w-4 h-4 shrink-0 text-emerald-400" />
      <span>{{ toastMessage.text }}</span>
    </div>

    <!-- Header & Security Status -->
    <header class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div>
        <h1 class="text-2xl sm:text-3xl font-extrabold tracking-tight text-white font-outfit">
          Diário Criptografado
        </h1>
        <p class="text-xs sm:text-sm text-slate-400 mt-1">
          Escrita reflexiva com garantia de Conhecimento Zero (Zero-Knowledge): cifragem local com AES-256-GCM e Argon2id.
        </p>
      </div>

      <div class="self-start sm:self-auto">
        <LockStatusBadge
          :is-unlocked="vaultStore.isUnlocked"
          :loading="vaultStore.loading"
          @unlock="isUnlockModalOpen = true"
          @lock="handleLock"
        />
      </div>
    </header>

    <!-- Locked Vault Banner (When Vault is Locked) -->
    <section
      v-if="!vaultStore.isUnlocked"
      class="p-6 sm:p-10 rounded-2xl bg-surface-card border border-surface-border text-center space-y-5 shadow-lg glow-card"
    >
      <div class="w-12 h-12 sm:w-14 sm:h-14 mx-auto rounded-2xl bg-zinc-800 flex items-center justify-center border border-zinc-700 text-zinc-100 shadow-sm">
        <Lock class="w-6 h-6 sm:w-7 sm:h-7" />
      </div>
      <div class="max-w-md mx-auto space-y-2">
        <h2 class="text-lg sm:text-xl font-bold text-white font-outfit">Cofre de Reflexões Protegido</h2>
        <p class="text-xs text-zinc-400 leading-relaxed">
          Suas anotações são armazenadas cifradas no servidor. Para escrever ou ler reflexões, desbloqueie seu cofre fornecendo a senha mestre para derivação da chave em memória.
        </p>
      </div>
      <div>
        <button
          type="button"
          @click="isUnlockModalOpen = true"
          class="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-zinc-100 hover:bg-white text-sm font-semibold text-zinc-950 shadow-sm transition-all touch-target"
        >
          <KeyRound class="w-4 h-4" />
          <span>Desbloquear Diário</span>
        </button>
      </div>
    </section>

    <!-- Unlocked Editor Workspace (When Vault is Unlocked) -->
    <section v-else class="space-y-5 sm:space-y-6">
      <!-- Date Navigation Toolbar -->
      <div class="flex flex-wrap items-center justify-between gap-2.5 p-2.5 sm:p-3 rounded-2xl bg-surface-card border border-surface-border">
        <div class="flex items-center gap-1.5 sm:gap-2">
          <button
            type="button"
            @click="changeDate(-1)"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
            title="Dia anterior"
          >
            <ChevronLeft class="w-4 h-4" />
          </button>
          <span class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-surface-dark border border-surface-border text-xs font-semibold text-white">
            <Calendar class="w-3.5 h-3.5 text-brand-400 shrink-0" />
            <span class="capitalize truncate max-w-[150px] sm:max-w-none">{{ formattedDateDisplay }}</span>
          </span>
          <button
            type="button"
            @click="changeDate(1)"
            class="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
            title="Próximo dia"
          >
            <ChevronRight class="w-4 h-4" />
          </button>
        </div>

        <div class="flex items-center gap-1.5 sm:gap-2">
          <button
            v-if="hasEntryForCurrentDate"
            type="button"
            @click="handleDeleteEntry"
            title="Excluir reflexão deste dia"
            class="p-2 rounded-xl text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all touch-target sm:min-h-0 sm:min-w-0"
          >
            <Trash2 class="w-4 h-4" />
          </button>

          <button
            type="button"
            @click="setToday"
            class="px-3 py-1.5 rounded-xl text-xs font-semibold text-slate-300 hover:text-white bg-surface-dark border border-surface-border hover:bg-surface-border/60 transition-colors touch-target sm:min-h-0 sm:min-w-0"
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
        @update:plaintext-reflection="
          (val) => {
            plaintextReflection = val
            vaultStore.resetInactivityTimer()
          }
        "
        @update:mood-score="
          (val) => {
            moodScore = val
            vaultStore.resetInactivityTimer()
          }
        "
        @update:tags="
          (val) => {
            tags = val
            vaultStore.resetInactivityTimer()
          }
        "
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
