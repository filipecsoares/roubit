<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Download, Share2, PlusSquare, X, ShieldCheck } from 'lucide-vue-next'
import { usePwa } from '@/composables/usePwa'

const { isInstallable, isInstalled, installApp } = usePwa()

const isDismissed = ref(false)
const showIosGuide = ref(false)
const isIos = ref(false)

const DISMISS_KEY = 'roubit_pwa_install_dismissed'
const DISMISS_DAYS = 7

onMounted(() => {
  if (typeof window === 'undefined') return

  // Detect iOS Safari
  const ua = window.navigator.userAgent
  isIos.value = /iphone|ipad|ipod/i.test(ua) && !(window as any).MSStream

  // Check dismissal timestamp
  const dismissedAt = localStorage.getItem(DISMISS_KEY)
  if (dismissedAt) {
    const elapsed = Date.now() - parseInt(dismissedAt, 10)
    if (elapsed < DISMISS_DAYS * 24 * 60 * 60 * 1000) {
      isDismissed.value = true
    }
  }
})

const shouldShow = computed(() => {
  if (isDismissed.value || isInstalled.value) return false
  if (isInstallable.value) return true
  if (isIos.value && !isInstalled.value) return true
  return false
})

function dismiss() {
  isDismissed.value = true
  localStorage.setItem(DISMISS_KEY, Date.now().toString())
}

async function handleInstall() {
  if (isIos.value) {
    showIosGuide.value = !showIosGuide.value
    return
  }
  const installed = await installApp()
  if (installed) {
    dismiss()
  }
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
      v-if="shouldShow"
      class="fixed bottom-20 md:bottom-8 left-4 right-4 sm:left-auto sm:right-6 sm:max-w-md z-40 bg-zinc-900/95 border border-zinc-700/80 rounded-2xl shadow-2xl p-4 backdrop-blur-xl text-zinc-100 flex flex-col gap-3"
      role="region"
      aria-label="Instalar Aplicativo Roubit"
    >
      <div class="flex items-start justify-between gap-3">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-zinc-800 border border-zinc-700 flex items-center justify-center text-zinc-100 shadow-sm shrink-0">
            <ShieldCheck class="w-5 h-5 text-emerald-400" />
          </div>
          <div class="flex flex-col">
            <h4 class="text-xs sm:text-sm font-bold text-white tracking-tight">Instalar Aplicativo Roubit</h4>
            <p class="text-[11px] text-zinc-400 leading-tight">
              Acesso offline instantâneo, hábitos e modo foco na tela inicial.
            </p>
          </div>
        </div>

        <button
          type="button"
          @click="dismiss"
          class="p-1 rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          title="Dispensar sugestão"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- iOS Installation Steps Guide -->
      <div v-if="showIosGuide && isIos" class="p-3 rounded-xl bg-zinc-800/80 border border-zinc-700 text-xs flex flex-col gap-2">
        <div class="flex items-center gap-2 text-zinc-200 font-medium">
          <Share2 class="w-4 h-4 text-brand-400 shrink-0" />
          <span>1. Toque no botão de <strong>Compartilhar</strong> no Safari</span>
        </div>
        <div class="flex items-center gap-2 text-zinc-200 font-medium">
          <PlusSquare class="w-4 h-4 text-emerald-400 shrink-0" />
          <span>2. Role para baixo e selecione <strong>Adicionar à Tela de Início</strong></span>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2 pt-1 border-t border-zinc-800">
        <button
          type="button"
          @click="dismiss"
          class="px-3 py-1.5 rounded-xl text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50 transition-colors"
        >
          Mais tarde
        </button>

        <button
          type="button"
          @click="handleInstall"
          class="flex items-center gap-1.5 px-4 py-1.5 rounded-xl bg-zinc-100 hover:bg-white text-zinc-900 text-xs font-bold shadow-md active:scale-95 transition-all"
        >
          <Download v-if="!isIos" class="w-3.5 h-3.5" />
          <Share2 v-else class="w-3.5 h-3.5" />
          <span>{{ isIos ? (showIosGuide ? 'Ocultar Guia' : 'Como Instalar') : 'Instalar Agora' }}</span>
        </button>
      </div>
    </div>
  </transition>
</template>
