<script setup lang="ts">
import { Lock, Unlock, ShieldAlert, ShieldCheck } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    isUnlocked: boolean
    loading?: boolean
  }>(),
  {
    isUnlocked: false,
    loading: false,
  }
)

const emit = defineEmits<{
  (e: 'unlock'): void
  (e: 'lock'): void
}>()
</script>

<template>
  <div class="inline-flex items-center gap-2">
    <!-- Unlocked State -->
    <div
      v-if="isUnlocked"
      class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-medium shadow-sm"
    >
      <ShieldCheck class="w-4 h-4 text-emerald-400" />
      <span>Cofre Desbloqueado (Chave na RAM)</span>
      <button
        type="button"
        @click="emit('lock')"
        class="ml-2 px-2 py-0.5 rounded-lg bg-surface-border/60 hover:bg-rose-500/20 text-slate-300 hover:text-rose-400 border border-surface-border transition-colors text-[11px] font-semibold"
        title="Bloquear cofre e limpar chave da memória"
      >
        <Lock class="w-3 h-3 inline mr-1" />
        Bloquear
      </button>
    </div>

    <!-- Locked State -->
    <div
      v-else
      class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-slate-800/80 border border-surface-border text-slate-400 text-xs font-medium"
    >
      <ShieldAlert class="w-4 h-4 text-amber-400" />
      <span>Cofre Bloqueado</span>
      <button
        type="button"
        @click="emit('unlock')"
        :disabled="loading"
        class="ml-2 px-2.5 py-0.5 rounded-lg bg-brand-600 hover:bg-brand-500 text-white font-semibold transition-colors text-[11px] shadow-sm"
      >
        <Unlock class="w-3 h-3 inline mr-1" />
        Desbloquear
      </button>
    </div>
  </div>
</template>
