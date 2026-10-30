<script setup lang="ts">
import AppNavbar from '@/components/common/AppNavbar.vue'
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
