<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { LayoutDashboard, CheckCircle2, Target, BookOpen, LogOut, Shield, Timer } from 'lucide-vue-next'
import { useFocusStore } from '@/stores/focus'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const focusStore = useFocusStore()

const isLoginPage = computed(() => route.name === 'login')

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'Hábitos', path: '/habits', icon: CheckCircle2 },
  { name: 'OKRs', path: '/okrs', icon: Target },
  { name: 'Diário', path: '/journal', icon: BookOpen },
]

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<template>
  <header v-if="!isLoginPage" class="sticky top-0 z-40 glass-panel border-b border-surface-border">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex items-center justify-between h-14 sm:h-16">
        <!-- Logo Brand -->
        <router-link to="/" class="flex items-center gap-2.5 group select-none">
          <div class="w-8 h-8 sm:w-9 sm:h-9 rounded-xl bg-zinc-800 border border-zinc-700/80 flex items-center justify-center text-zinc-100 shadow-sm group-hover:border-zinc-500 transition-colors">
            <Shield class="w-4 h-4 sm:w-5 sm:h-5" />
          </div>
          <div class="flex flex-col">
            <div class="flex items-center gap-1.5">
              <span class="text-base sm:text-lg font-black tracking-tight text-white font-outfit">Roubit</span>
              <span class="text-[9px] sm:text-[10px] text-zinc-400 font-mono bg-zinc-800 px-1.5 py-0.5 rounded-full border border-zinc-700">v0.1.0</span>
            </div>
            <span class="text-[10px] text-zinc-400 hidden sm:block -mt-1 font-sans">Rotinas, Hábitos & Diário</span>
          </div>
        </router-link>

        <!-- Desktop Navigation Links -->
        <nav class="hidden md:flex items-center gap-1 bg-surface-card p-1 rounded-xl border border-surface-border">
          <router-link
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            class="flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs sm:text-sm font-medium transition-all"
            :class="
              route.path === item.path
                ? 'bg-zinc-800 text-zinc-100 border border-zinc-700 shadow-sm'
                : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40 border border-transparent'
            "
          >
            <component :is="item.icon" class="w-4 h-4" />
            <span>{{ item.name }}</span>
          </router-link>
        </nav>

        <!-- Right Side Actions -->
        <div class="flex items-center gap-2">
          <!-- Focus Mode Quick Trigger -->
          <button
            type="button"
            @click="focusStore.openFocusModal()"
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all touch-target"
            :class="
              focusStore.isRunning
                ? 'bg-zinc-800 text-emerald-400 border border-emerald-500/40 shadow-sm'
                : 'text-zinc-300 hover:text-white bg-surface-card hover:bg-zinc-800 border border-surface-border'
            "
            title="Abrir Modo Foco & Timer Pomodoro"
          >
            <Timer class="w-4 h-4" :class="focusStore.isRunning ? 'text-emerald-400' : 'text-zinc-400'" />
            <span v-if="focusStore.isRunning" class="font-mono font-bold">{{ focusStore.formattedTime }}</span>
            <span v-else class="hidden sm:inline">Modo Foco</span>
          </button>

          <!-- User / Status Badge (desktop) -->
          <div class="hidden lg:flex items-center gap-2 px-2.5 py-1 rounded-full bg-zinc-800/80 border border-zinc-700/80 text-zinc-300 text-xs">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
            <span>Online</span>
          </div>

          <!-- Logout Button -->
          <button
            @click="handleLogout"
            title="Sair da conta"
            class="p-2 rounded-xl text-zinc-400 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all touch-target"
          >
            <LogOut class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  </header>

  <!-- Mobile Fixed Bottom Navigation Bar (Thumb Friendly) -->
  <nav
    v-if="!isLoginPage"
    class="fixed bottom-0 inset-x-0 z-50 md:hidden bg-surface-card/95 backdrop-blur-xl border-t border-surface-border shadow-[0_-8px_24px_rgba(0,0,0,0.5)] pb-[env(safe-area-inset-bottom)]"
  >
    <div class="grid grid-cols-4 items-center h-16 px-1">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="flex flex-col items-center justify-center h-full py-1 text-center select-none transition-all active:scale-95"
        :class="
          route.path === item.path
            ? 'text-white font-semibold'
            : 'text-zinc-400 hover:text-zinc-200 font-medium'
        "
      >
        <div
          class="p-1 rounded-xl transition-all"
          :class="route.path === item.path ? 'bg-zinc-800 text-white' : 'text-zinc-400'"
        >
          <component
            :is="item.icon"
            class="w-5 h-5 transition-transform"
            :class="route.path === item.path ? 'scale-105' : ''"
          />
        </div>
        <span class="text-[11px] tracking-tight mt-0.5">{{ item.name }}</span>
      </router-link>
    </div>
  </nav>
</template>
