<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { LayoutDashboard, CheckCircle2, Target, BookOpen, LogOut, Shield } from 'lucide-vue-next'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

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
          <div class="w-8 h-8 sm:w-9 sm:h-9 rounded-xl bg-gradient-to-tr from-brand-600 via-indigo-600 to-indigo-400 flex items-center justify-center text-white shadow-md shadow-brand-500/25 group-hover:scale-105 transition-transform">
            <Shield class="w-4 h-4 sm:w-5 sm:h-5" />
          </div>
          <div class="flex flex-col">
            <div class="flex items-center gap-1.5">
              <span class="text-base sm:text-lg font-black tracking-tight text-white font-outfit">Roubit</span>
              <span class="text-[9px] sm:text-[10px] text-brand-300 font-mono bg-brand-500/10 px-1.5 py-0.2 rounded-full border border-brand-500/20">v0.1.0</span>
            </div>
            <span class="text-[10px] text-slate-400 hidden sm:block -mt-1 font-sans">Rotinas, Hábitos & Diário</span>
          </div>
        </router-link>

        <!-- Desktop Navigation Links -->
        <nav class="hidden md:flex items-center gap-1.5 bg-surface-dark/60 p-1 rounded-2xl border border-surface-border/60">
          <router-link
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            class="flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs sm:text-sm font-medium transition-all"
            :class="
              route.path === item.path
                ? 'bg-brand-500/15 text-brand-300 border border-brand-500/25 shadow-sm'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/40 border border-transparent'
            "
          >
            <component :is="item.icon" class="w-4 h-4" />
            <span>{{ item.name }}</span>
          </router-link>
        </nav>

        <!-- Right Side Actions -->
        <div class="flex items-center gap-2">
          <!-- User / Status Badge (desktop) -->
          <div class="hidden lg:flex items-center gap-2 px-2.5 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>Online</span>
          </div>

          <!-- Logout Button -->
          <button
            @click="handleLogout"
            title="Sair da conta"
            class="p-2 rounded-xl text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 border border-transparent hover:border-rose-500/20 transition-all touch-target"
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
    class="fixed bottom-0 inset-x-0 z-50 md:hidden bg-surface-card/90 backdrop-blur-xl border-t border-surface-border/80 shadow-[0_-8px_24px_rgba(0,0,0,0.4)] pb-[env(safe-area-inset-bottom)]"
  >
    <div class="grid grid-cols-4 items-center h-16 px-1">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="flex flex-col items-center justify-center h-full py-1 text-center select-none transition-all active:scale-95"
        :class="
          route.path === item.path
            ? 'text-brand-400 font-semibold'
            : 'text-slate-400 hover:text-slate-200 font-medium'
        "
      >
        <div
          class="p-1 rounded-xl transition-all"
          :class="route.path === item.path ? 'bg-brand-500/15' : ''"
        >
          <component
            :is="item.icon"
            class="w-5 h-5 transition-transform"
            :class="route.path === item.path ? 'scale-110' : ''"
          />
        </div>
        <span class="text-[11px] tracking-tight mt-0.5">{{ item.name }}</span>
      </router-link>
    </div>
  </nav>
</template>
