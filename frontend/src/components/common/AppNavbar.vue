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
  <nav v-if="!isLoginPage" class="sticky top-0 z-50 glass-panel border-b border-surface-border">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex items-center justify-between h-16">
        <!-- Logo -->
        <router-link to="/" class="flex items-center gap-2.5 group">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-tr from-brand-600 to-indigo-400 flex items-center justify-center text-white shadow-md shadow-brand-500/20 group-hover:scale-105 transition-transform">
            <Shield class="w-5 h-5" />
          </div>
          <div class="flex flex-col">
            <span class="text-lg font-black tracking-tight text-white font-outfit">Roubit</span>
            <span class="text-[10px] text-slate-400 font-mono -mt-1">v0.1.0</span>
          </div>
        </router-link>

        <!-- Navigation Links -->
        <div class="hidden md:flex items-center gap-1">
          <router-link
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            class="flex items-center gap-2 px-3.5 py-2 rounded-xl text-sm font-medium transition-all"
            :class="route.path === item.path ? 'bg-brand-500/10 text-brand-400 border border-brand-500/20 shadow-sm' : 'text-slate-400 hover:text-white hover:bg-slate-800/40'"
          >
            <component :is="item.icon" class="w-4 h-4" />
            <span>{{ item.name }}</span>
          </router-link>
        </div>

        <!-- Right Side Actions -->
        <div class="flex items-center gap-3">
          <button
            @click="handleLogout"
            title="Sair da conta"
            class="p-2 rounded-xl text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
          >
            <LogOut class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Mobile Navigation Bar -->
    <div class="md:hidden flex items-center justify-around py-2 border-t border-surface-border bg-surface-card/60 backdrop-blur-md">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="flex flex-col items-center gap-1 p-2 rounded-lg text-xs"
        :class="route.path === item.path ? 'text-brand-400 font-semibold' : 'text-slate-400'"
      >
        <component :is="item.icon" class="w-5 h-5" />
        <span>{{ item.name }}</span>
      </router-link>
    </div>
  </nav>
</template>
