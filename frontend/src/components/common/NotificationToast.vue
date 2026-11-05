<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { Bell, Calendar, Flame, X, Play } from 'lucide-vue-next'
import { useNotificationStore } from '@/stores/notification'
import { useFocusStore } from '@/stores/focus'
import type { AppNotification } from '@/types'

const notificationStore = useNotificationStore()
const focusStore = useFocusStore()

interface ActiveToast {
  notification: AppNotification
  timerId: ReturnType<typeof setTimeout>
}

const activeToasts = ref<ActiveToast[]>([])

// Watch for new notifications added to the store and trigger a toast
watch(
  () => notificationStore.recentNotifications[0],
  (newNotif) => {
    if (!newNotif || newNotif.read) return

    // Avoid duplicate toast if already showing
    if (activeToasts.value.some((t) => t.notification.id === newNotif.id)) return

    const timerId = setTimeout(() => {
      dismissToast(newNotif.id)
    }, 6000)

    activeToasts.value.push({
      notification: newNotif,
      timerId,
    })

    // Limit maximum visible stacked toasts to 4
    if (activeToasts.value.length > 4) {
      const oldest = activeToasts.value.shift()
      if (oldest) clearTimeout(oldest.timerId)
    }
  }
)

function dismissToast(id: string) {
  const index = activeToasts.value.findIndex((t) => t.notification.id === id)
  if (index !== -1) {
    clearTimeout(activeToasts.value[index].timerId)
    activeToasts.value.splice(index, 1)
  }
  notificationStore.markAsRead(id)
}

function handleQuickAction(toast: ActiveToast) {
  const notif = toast.notification
  if (notif.entity_type === 'time_block') {
    focusStore.openFocusModal({
      id: notif.entity_id,
      name: notif.title.replace(/^Rotina:\s*/, ''),
    })
  }
  dismissToast(notif.id)
}

onUnmounted(() => {
  for (const toast of activeToasts.value) {
    clearTimeout(toast.timerId)
  }
})
</script>

<template>
  <aside
    aria-label="Notificações em tempo real"
    role="status"
    aria-live="polite"
    class="fixed top-16 sm:top-20 right-3 sm:right-6 z-50 flex flex-col gap-2.5 max-w-sm w-full pointer-events-none"
  >
    <TransitionGroup name="toast">
      <div
        v-for="toast in activeToasts"
        :key="toast.notification.id"
        class="pointer-events-auto bg-[#121215]/95 border border-zinc-800 shadow-2xl rounded-2xl p-4 flex flex-col gap-2 backdrop-blur-xl text-zinc-200 hover:border-zinc-700 transition-all"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="flex items-start gap-3">
            <div
              class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 border mt-0.5"
              :class="[
                toast.notification.entity_type === 'time_block'
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                  : toast.notification.entity_type === 'habit'
                  ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                  : 'bg-zinc-800 text-zinc-300 border-zinc-700'
              ]"
            >
              <Calendar v-if="toast.notification.entity_type === 'time_block'" class="w-4 h-4" />
              <Flame v-else-if="toast.notification.entity_type === 'habit'" class="w-4 h-4" />
              <Bell v-else class="w-4 h-4" />
            </div>

            <div class="flex flex-col">
              <span class="text-xs sm:text-sm font-bold text-white tracking-tight line-clamp-1">
                {{ toast.notification.title }}
              </span>
              <p class="text-xs text-zinc-400 line-clamp-2 mt-0.5 leading-relaxed">
                {{ toast.notification.message }}
              </p>
            </div>
          </div>

          <button
            type="button"
            @click="dismissToast(toast.notification.id)"
            class="text-zinc-500 hover:text-zinc-300 p-1 rounded-lg hover:bg-zinc-800/80 transition-colors shrink-0"
            title="Fechar notificação"
            aria-label="Fechar notificação"
          >
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Quick Action Trigger -->
        <div v-if="toast.notification.entity_type === 'time_block'" class="flex items-center justify-end gap-2 pt-1 border-t border-zinc-800/60 mt-1">
          <button
            type="button"
            @click="handleQuickAction(toast)"
            class="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 text-xs font-semibold transition-all active:scale-95"
          >
            <Play class="w-3 h-3 fill-current" />
            <span>Iniciar Foco</span>
          </button>
        </div>
      </div>
    </TransitionGroup>
  </aside>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(30px) scale(0.95);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(20px) scale(0.92);
}
</style>
