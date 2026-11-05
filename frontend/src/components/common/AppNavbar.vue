<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { LayoutDashboard, CheckCircle2, Target, BookOpen, LogOut, Shield, Timer, Bell, CheckCheck, Calendar, Flame, Play, WifiOff, RefreshCw } from 'lucide-vue-next'
import { useFocusStore } from '@/stores/focus'
import { useNotificationStore } from '@/stores/notification'
import { usePwa } from '@/composables/usePwa'
import type { AppNotification } from '@/types'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const focusStore = useFocusStore()
const notificationStore = useNotificationStore()
const { isOnline, pendingSyncCount, isSyncing, recentlySynced } = usePwa()

const isLoginPage = computed(() => route.name === 'login')
const isDropdownOpen = ref(false)

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'Hábitos', path: '/habits', icon: CheckCircle2 },
  { name: 'OKRs', path: '/okrs', icon: Target },
  { name: 'Diário', path: '/journal', icon: BookOpen },
]

async function handleLogout() {
  await authStore.logout()
  router.push('/login')
}

function formatRelativeTime(isoString: string): string {
  try {
    const diff = (Date.now() - new Date(isoString).getTime()) / 1000
    if (diff < 60) return 'agora'
    if (diff < 3600) return `há ${Math.floor(diff / 60)} min`
    if (diff < 86400) return `há ${Math.floor(diff / 3600)} h`
    return new Date(isoString).toLocaleDateString('pt-BR', { day: '2-digit', month: '2-digit' })
  } catch {
    return 'recente'
  }
}

function handleNotificationAction(notif: AppNotification) {
  notificationStore.markAsRead(notif.id)
  if (notif.entity_type === 'time_block') {
    focusStore.openFocusModal({
      id: notif.entity_id,
      name: notif.title.replace(/^Rotina:\s*/, ''),
    })
  }
  isDropdownOpen.value = false
}

function closeDropdownOnClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement | null
  if (isDropdownOpen.value && target && !target.closest('.notification-dropdown-container')) {
    isDropdownOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', closeDropdownOnClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', closeDropdownOnClickOutside)
})
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
          <!-- PWA Network Status Pill (Visible when Offline, Syncing or Recently Synced) -->
          <div
            v-if="!isOnline || pendingSyncCount > 0 || isSyncing || recentlySynced"
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-xs font-medium border transition-all"
            :class="
              recentlySynced
                ? 'bg-emerald-950/60 text-emerald-400 border-emerald-500/40'
                : isSyncing
                  ? 'bg-sky-950/60 text-sky-400 border-sky-500/40'
                  : 'bg-amber-950/60 text-amber-400 border-amber-500/40'
            "
            :title="
              recentlySynced
                ? 'Dados sincronizados com o servidor'
                : isSyncing
                  ? 'Sincronizando mutações pendentes...'
                  : 'Modo offline: alterações serão salvas localmente e sincronizadas quando restabelecer a conexão'
            "
          >
            <RefreshCw v-if="isSyncing" class="w-3.5 h-3.5 animate-spin" />
            <CheckCircle2 v-else-if="recentlySynced" class="w-3.5 h-3.5 text-emerald-400" />
            <WifiOff v-else class="w-3.5 h-3.5 text-amber-400" />

            <span class="text-[11px] font-mono">
              {{
                recentlySynced
                  ? 'Sincronizado'
                  : isSyncing
                    ? 'Sincronizando...'
                    : pendingSyncCount > 0
                      ? `Offline (${pendingSyncCount})`
                      : 'Offline'
              }}
            </span>
          </div>

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

          <!-- Notification Bell Trigger & Dropdown -->
          <div class="relative notification-dropdown-container">
            <button
              type="button"
              @click="isDropdownOpen = !isDropdownOpen"
              class="relative p-2 rounded-xl text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/80 border border-transparent transition-all touch-target"
              :class="{ 'bg-zinc-800 text-zinc-100 border-zinc-700': isDropdownOpen }"
              title="Notificações em tempo real"
            >
              <Bell class="w-4 h-4" />
              <span
                v-if="notificationStore.unreadCount > 0"
                class="absolute -top-0.5 -right-0.5 flex h-4 min-w-4 px-1 items-center justify-center rounded-full bg-emerald-500 text-[9px] font-black text-black ring-2 ring-zinc-950 animate-pulse"
              >
                {{ notificationStore.unreadCount > 9 ? '9+' : notificationStore.unreadCount }}
              </span>
            </button>

            <!-- Dropdown Feed Menu -->
            <div
              v-if="isDropdownOpen"
              class="absolute right-0 mt-2 w-80 sm:w-96 rounded-2xl bg-[#121215] border border-zinc-800 shadow-2xl backdrop-blur-xl p-3 z-50 flex flex-col gap-2.5 text-zinc-200"
            >
              <div class="flex items-center justify-between pb-2 border-b border-zinc-800">
                <div class="flex items-center gap-2">
                  <span class="text-xs font-bold text-white uppercase tracking-wider font-mono">Notificações</span>
                  <span
                    v-if="notificationStore.unreadCount > 0"
                    class="text-[10px] bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 px-1.5 py-0.5 rounded-full font-mono font-bold"
                  >
                    {{ notificationStore.unreadCount }} nova{{ notificationStore.unreadCount > 1 ? 's' : '' }}
                  </span>
                </div>

                <button
                  v-if="notificationStore.recentNotifications.length > 0"
                  type="button"
                  @click="notificationStore.markAllAsRead()"
                  class="flex items-center gap-1 text-[11px] text-zinc-400 hover:text-emerald-400 transition-colors"
                >
                  <CheckCheck class="w-3.5 h-3.5" />
                  <span>Marcar lidas</span>
                </button>
              </div>

              <!-- Permission Prompt Banner if default -->
              <div
                v-if="notificationStore.permissionState === 'default'"
                class="bg-zinc-900/90 border border-zinc-800 rounded-xl p-2.5 flex items-center justify-between gap-2"
              >
                <span class="text-[11px] text-zinc-300">Receber alertas no navegador</span>
                <button
                  type="button"
                  @click="notificationStore.requestSystemPermission()"
                  class="px-2.5 py-1 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-black text-[10px] font-bold tracking-tight transition-all shrink-0 active:scale-95"
                >
                  Ativar
                </button>
              </div>

              <!-- Notification Items List -->
              <div class="flex flex-col gap-1 max-h-72 overflow-y-auto pr-1">
                <div
                  v-if="notificationStore.recentNotifications.length === 0"
                  class="py-8 text-center text-xs text-zinc-500"
                >
                  Nenhuma notificação recente
                </div>

                <div
                  v-for="notif in notificationStore.recentNotifications.slice(0, 5)"
                  :key="notif.id"
                  @click="handleNotificationAction(notif)"
                  class="p-2.5 rounded-xl border transition-all cursor-pointer flex items-start gap-2.5 group"
                  :class="[
                    notif.read
                      ? 'bg-zinc-900/40 border-transparent hover:bg-zinc-800/50 hover:border-zinc-700/60'
                      : 'bg-zinc-900/90 border-zinc-800 hover:border-zinc-700 shadow-sm'
                  ]"
                >
                  <div
                    class="w-7 h-7 rounded-lg flex items-center justify-center shrink-0 border mt-0.5"
                    :class="[
                      notif.entity_type === 'time_block'
                        ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                        : notif.entity_type === 'habit'
                        ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                        : 'bg-zinc-800 text-zinc-300 border-zinc-700'
                    ]"
                  >
                    <Calendar v-if="notif.entity_type === 'time_block'" class="w-3.5 h-3.5" />
                    <Flame v-else-if="notif.entity_type === 'habit'" class="w-3.5 h-3.5" />
                    <Bell v-else class="w-3.5 h-3.5" />
                  </div>

                  <div class="flex flex-col flex-1 min-w-0">
                    <div class="flex items-center justify-between gap-1">
                      <span
                        class="text-xs font-semibold tracking-tight truncate"
                        :class="notif.read ? 'text-zinc-300' : 'text-white'"
                      >
                        {{ notif.title }}
                      </span>
                      <span class="text-[10px] text-zinc-500 shrink-0 font-mono">
                        {{ formatRelativeTime(notif.created_at) }}
                      </span>
                    </div>

                    <p class="text-[11px] text-zinc-400 line-clamp-2 mt-0.5">
                      {{ notif.message }}
                    </p>

                    <div v-if="notif.entity_type === 'time_block'" class="mt-1.5 flex justify-end">
                      <button
                        type="button"
                        class="flex items-center gap-1 text-[10px] font-bold text-emerald-400 hover:text-emerald-300 transition-colors"
                      >
                        <Play class="w-2.5 h-2.5 fill-current" />
                        <span>Focar no bloco</span>
                      </button>
                    </div>
                  </div>

                  <span
                    v-if="!notif.read"
                    class="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0 mt-1.5"
                  ></span>
                </div>
              </div>
            </div>
          </div>

          <!-- User / Status Badge (desktop) -->
          <div class="hidden lg:flex items-center gap-2 px-2.5 py-1 rounded-full bg-zinc-800/80 border border-zinc-700/80 text-zinc-300 text-xs">
            <span class="w-1.5 h-1.5 rounded-full" :class="isOnline ? 'bg-emerald-400' : 'bg-amber-400'"></span>
            <span>{{ isOnline ? 'Online' : 'Offline' }}</span>
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
