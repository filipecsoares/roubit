<script setup lang="ts">
import { ref } from 'vue'
import { Lock, X, Eye, EyeOff, KeyRound, AlertCircle } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    isOpen: boolean
    loading?: boolean
    errorMessage?: string | null
  }>(),
  {
    loading: false,
    errorMessage: null,
  }
)

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'unlock', passphrase: string): void
}>()

const passphrase = ref('')
const showPassword = ref(false)

function handleUnlock() {
  if (!passphrase.value.trim() || props.loading) return
  emit('unlock', passphrase.value)
}

function handleClose() {
  passphrase.value = ''
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-fade-in"
      @click="handleClose"
    >
      <div
        class="w-full max-w-md bg-surface-card border border-surface-border rounded-2xl shadow-2xl p-6 space-y-5"
        @click.stop
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-surface-border/60 pb-4">
          <div class="flex items-center gap-3">
            <div class="p-2.5 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20">
              <Lock class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-lg font-bold text-white">Desbloquear Diário</h3>
              <p class="text-xs text-slate-400">Criptografia de Conhecimento Zero</p>
            </div>
          </div>
          <button
            type="button"
            @click="handleClose"
            class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-surface-border/60 transition-colors"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <p class="text-xs text-slate-400 leading-relaxed">
          Insira sua senha mestre para derivar a chave de decifragem <strong>AES-256-GCM</strong> localmente via <strong>Argon2id WASM</strong>. Sua senha nunca trafega pela rede nem é salva em disco.
        </p>

        <!-- Error Banner -->
        <div
          v-if="errorMessage"
          class="flex items-center gap-2 p-3 rounded-xl bg-rose-950/60 border border-rose-500/40 text-xs text-rose-300"
        >
          <AlertCircle class="w-4 h-4 shrink-0 text-rose-400" />
          <span>{{ errorMessage }}</span>
        </div>

        <!-- Passphrase Input -->
        <div class="space-y-1.5">
          <label class="block text-xs font-semibold text-slate-300 uppercase tracking-wider">
            Senha Mestre / Passphrase
          </label>
          <div class="relative">
            <input
              v-model="passphrase"
              :type="showPassword ? 'text' : 'password'"
              placeholder="Digite sua senha mestre..."
              autofocus
              :disabled="loading"
              @keyup.enter="handleUnlock"
              class="w-full pl-3.5 pr-10 py-2.5 rounded-xl bg-surface-dark border border-surface-border focus:border-brand-500 focus:ring-1 focus:ring-brand-500 text-sm text-white placeholder-slate-500 outline-none transition-all"
            />
            <button
              type="button"
              @click="showPassword = !showPassword"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white transition-colors"
            >
              <EyeOff v-if="showPassword" class="w-4 h-4" />
              <Eye v-else class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Actions -->
        <div class="space-y-2 pt-2">
          <button
            type="button"
            :disabled="!passphrase.trim() || loading"
            @click="handleUnlock"
            class="w-full py-2.5 px-4 rounded-xl bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-sm font-semibold text-white shadow-lg shadow-brand-500/25 transition-all flex items-center justify-center gap-2"
          >
            <KeyRound class="w-4 h-4" :class="{ 'animate-spin': loading }" />
            <span>{{ loading ? 'Derivando Chaves (Argon2id)...' : 'Desbloquear Reflexões' }}</span>
          </button>
          <button
            type="button"
            @click="handleClose"
            class="w-full py-2 text-xs font-medium text-slate-400 hover:text-white transition-colors"
          >
            Cancelar
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
