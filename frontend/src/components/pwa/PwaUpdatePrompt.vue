<script setup lang="ts">
import { Sparkles, RefreshCw, X } from 'lucide-vue-next'
import { usePwa } from '@/composables/usePwa'

const { needRefresh, refreshApp } = usePwa()

function handleDismiss() {
  needRefresh.value = false
}
</script>

<template>
  <transition
    enter-active-class="transform ease-out duration-300 transition"
    enter-from-class="translate-y-4 opacity-0 sm:translate-y-0 sm:scale-95"
    enter-to-class="translate-y-0 opacity-100 sm:scale-100"
    leave-active-class="transition ease-in duration-200"
    leave-from-class="opacity-100"
    leave-to-class="opacity-0"
  >
    <div
      v-if="needRefresh"
      class="fixed bottom-24 md:bottom-8 right-4 sm:right-6 z-50 max-w-sm w-full bg-zinc-900/95 border border-brand-500/40 rounded-2xl shadow-2xl p-4 backdrop-blur-xl flex items-center justify-between gap-3 text-zinc-100"
      role="alert"
    >
      <div class="flex items-center gap-3">
        <div class="p-2 rounded-xl bg-brand-500/20 text-brand-400 border border-brand-500/30">
          <Sparkles class="w-5 h-5 animate-pulse" />
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-bold text-white tracking-wide">Atualização Pronta</span>
          <span class="text-[11px] text-zinc-400">Nova versão do Roubit disponível.</span>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          @click="refreshApp"
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-brand-600 hover:bg-brand-500 text-white text-xs font-semibold shadow-md active:scale-95 transition-all"
        >
          <RefreshCw class="w-3.5 h-3.5" />
          <span>Atualizar</span>
        </button>

        <button
          type="button"
          @click="handleDismiss"
          class="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          title="Fechar"
        >
          <X class="w-4 h-4" />
        </button>
      </div>
    </div>
  </transition>
</template>
