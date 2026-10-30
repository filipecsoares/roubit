<script setup lang="ts">
import { Timer } from 'lucide-vue-next'
import AppNavbar from '@/components/common/AppNavbar.vue'
import FocusModeModal from '@/components/dashboard/FocusModeModal.vue'
import { useFocusStore } from '@/stores/focus'

const focusStore = useFocusStore()
</script>

<template>
  <div class="min-h-screen bg-surface-dark flex flex-col selection:bg-brand-500 selection:text-white">
    <!-- Top Header and Bottom Bar -->
    <AppNavbar />

    <!-- Main Content View Area -->
    <main class="flex-1 max-w-7xl w-full mx-auto px-3 sm:px-6 lg:px-8 pt-3 sm:pt-6 pb-24 md:pb-10">
      <router-view v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>

    <!-- Global Floating Focus Pill (When timer is running in background) -->
    <div
      v-if="focusStore.isRunning && !focusStore.isModalOpen"
      @click="focusStore.openFocusModal()"
      class="fixed bottom-20 md:bottom-8 right-3 sm:right-6 z-40 bg-surface-card/95 border border-brand-500/50 shadow-2xl shadow-brand-500/20 rounded-2xl px-3 py-2 sm:px-4 sm:py-2.5 flex items-center gap-2.5 sm:gap-3 backdrop-blur-xl cursor-pointer hover:scale-105 active:scale-95 transition-all animate-bounce-short"
      title="Retornar ao Modo Foco"
    >
      <div class="p-1.5 sm:p-2 rounded-xl bg-brand-500/20 text-brand-300">
        <Timer class="w-3.5 h-3.5 sm:w-4 sm:h-4 animate-spin" style="animation-duration: 4s;" />
      </div>
      <div class="flex flex-col text-left">
        <span class="text-xs sm:text-sm font-bold text-white font-mono tracking-tight">{{ focusStore.formattedTime }}</span>
        <span class="text-[10px] text-brand-300 font-medium -mt-0.5 truncate max-w-[110px] sm:max-w-[140px]">
          {{ focusStore.isBreak ? 'Pausa Ativa' : (focusStore.associatedBlockName || 'Foco Livre') }}
        </span>
      </div>
    </div>

    <!-- Global Focus Mode Modal -->
    <FocusModeModal />
  </div>
</template>

<style>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease-out, transform 0.15s ease-out;
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
