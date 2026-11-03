import { ref, onMounted, onUnmounted } from 'vue'
import { pendingSyncCount, isSyncing, syncOfflineQueue } from '@/services/offlineQueue'
import { apiClient } from '@/services/api'
import { useRegisterSW } from 'virtual:pwa-register/vue'

export interface BeforeInstallPromptEvent extends Event {
  readonly platforms: string[]
  readonly userChoice: Promise<{
    outcome: 'accepted' | 'dismissed'
    platform: string
  }>
  prompt(): Promise<void>
}

// Global deferred prompt captured across composable instances
const deferredInstallPrompt = ref<BeforeInstallPromptEvent | null>(null)
const isInstallable = ref(false)
const isInstalled = ref(false)
const isOnline = ref(typeof window !== 'undefined' ? window.navigator.onLine : true)
const recentlySynced = ref(false)

export function usePwa() {
  const { needRefresh, updateServiceWorker } = useRegisterSW({
    immediate: true,
    onNeedRefresh() {
      needRefresh.value = true
    },
    onOfflineReady() {
      // Offline cache ready
    },
  })

  function handleOnline() {
    isOnline.value = true
    syncOfflineQueue(apiClient).then((res) => {
      if (res.synced > 0) {
        recentlySynced.value = true
        setTimeout(() => {
          recentlySynced.value = false
        }, 4000)
      }
    })
  }

  function handleOffline() {
    isOnline.value = false
  }

  function handleBeforeInstallPrompt(e: Event) {
    e.preventDefault()
    deferredInstallPrompt.value = e as BeforeInstallPromptEvent
    isInstallable.value = true
  }

  function handleAppInstalled() {
    isInstalled.value = true
    isInstallable.value = false
    deferredInstallPrompt.value = null
  }

  onMounted(() => {
    if (typeof window === 'undefined') return

    // Check if running in standalone mode (installed PWA)
    const isStandalone =
      window.matchMedia('(display-mode: standalone)').matches ||
      (window.navigator as any).standalone === true
    if (isStandalone) {
      isInstalled.value = true
    }

    window.addEventListener('online', handleOnline)
    window.addEventListener('offline', handleOffline)
    window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt)
    window.addEventListener('appinstalled', handleAppInstalled)
  })

  onUnmounted(() => {
    if (typeof window === 'undefined') return
    window.removeEventListener('online', handleOnline)
    window.removeEventListener('offline', handleOffline)
    window.removeEventListener('beforeinstallprompt', handleBeforeInstallPrompt)
    window.removeEventListener('appinstalled', handleAppInstalled)
  })

  /**
   * Prompts native installation if available.
   */
  async function installApp(): Promise<boolean> {
    if (!deferredInstallPrompt.value) {
      return false
    }

    try {
      await deferredInstallPrompt.value.prompt()
      const choice = await deferredInstallPrompt.value.userChoice
      if (choice.outcome === 'accepted') {
        isInstalled.value = true
        isInstallable.value = false
        deferredInstallPrompt.value = null
        return true
      }
      return false
    } catch {
      return false
    }
  }

  /**
   * Triggers Service Worker reload with skipWaiting.
   */
  async function refreshApp(): Promise<void> {
    await updateServiceWorker(true)
  }

  /**
   * Updates App Badging API with uncompleted task/habit counts.
   */
  function updateAppBadge(count: number): void {
    if (typeof navigator !== 'undefined' && 'setAppBadge' in navigator) {
      if (count > 0) {
        navigator.setAppBadge(count).catch(() => {})
      } else {
        navigator.clearAppBadge().catch(() => {})
      }
    }
  }

  /**
   * Clears native App Badge.
   */
  function clearAppBadge(): void {
    if (typeof navigator !== 'undefined' && 'clearAppBadge' in navigator) {
      navigator.clearAppBadge().catch(() => {})
    }
  }

  return {
    isOnline,
    isInstallable,
    isInstalled,
    needRefresh,
    pendingSyncCount,
    isSyncing,
    recentlySynced,
    installApp,
    refreshApp,
    updateAppBadge,
    clearAppBadge,
  }
}

export default usePwa
