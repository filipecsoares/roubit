import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useAuthStore } from './auth'
import type { AppNotification } from '@/types'

import { playNotificationChime } from '@/services/notificationAudio'

export const useNotificationStore = defineStore('notification', () => {
  const authStore = useAuthStore()

  const isConnected = ref(false)
  const recentNotifications = ref<AppNotification[]>([])
  const permissionState = ref<NotificationPermission>(
    typeof Notification !== 'undefined' ? Notification.permission : 'default'
  )

  let eventSource: EventSource | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let retryDelayMs = 1000
  const MAX_RETRY_DELAY_MS = 30000

  const unreadCount = computed(
    () => recentNotifications.value.filter((n) => !n.read).length
  )

  function addNotification(
    payload: Partial<AppNotification> & { title: string; message: string }
  ): AppNotification {
    const id = payload.id || (typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : `notif_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`)
    const notification: AppNotification = {
      id,
      user_id: payload.user_id,
      entity_type: payload.entity_type || 'system',
      entity_id: payload.entity_id || id,
      title: payload.title,
      message: payload.message,
      scheduled_at: payload.scheduled_at || new Date().toISOString(),
      read: false,
      created_at: payload.created_at || new Date().toISOString(),
    }

    // Keep up to 50 notifications in reverse chronological order
    recentNotifications.value.unshift(notification)
    if (recentNotifications.value.length > 50) {
      recentNotifications.value.pop()
    }

    // Play subtle audio chime
    playNotificationChime()

    // Dispatch native OS Notification when document is hidden and permission is granted
    if (
      typeof window !== 'undefined' &&
      typeof document !== 'undefined' &&
      document.hidden &&
      typeof Notification !== 'undefined' &&
      permissionState.value === 'granted'
    ) {
      try {
        const osNotif = new Notification(notification.title, {
          body: notification.message,
          icon: '/favicon.ico',
          tag: notification.id,
        })
        osNotif.onclick = () => {
          window.focus()
          osNotif.close()
        }
      } catch {
        // Fallback gracefully if notifications are blocked or failed
      }
    }

    return notification
  }

  function markAsRead(id: string): void {
    const notif = recentNotifications.value.find((n) => n.id === id)
    if (notif) {
      notif.read = true
    }
  }

  function markAllAsRead(): void {
    for (const notif of recentNotifications.value) {
      notif.read = true
    }
  }

  function removeNotification(id: string): void {
    recentNotifications.value = recentNotifications.value.filter((n) => n.id !== id)
  }

  function clearAll(): void {
    recentNotifications.value = []
  }

  async function requestSystemPermission(): Promise<NotificationPermission> {
    if (typeof Notification === 'undefined') {
      permissionState.value = 'denied'
      return 'denied'
    }

    try {
      const result = await Notification.requestPermission()
      permissionState.value = result
      return result
    } catch {
      permissionState.value = 'denied'
      return 'denied'
    }
  }

  function handleIncomingEvent(dataStr: string): void {
    try {
      const parsed = JSON.parse(dataStr)
      addNotification({
        id: parsed.id,
        user_id: parsed.user_id,
        entity_type: parsed.entity_type,
        entity_id: parsed.entity_id,
        title: parsed.title || 'Lembrete Roubit',
        message: parsed.message || 'Você tem uma nova notificação agendada.',
        scheduled_at: parsed.scheduled_at,
      })
    } catch {
      // Plaintext fallback
      addNotification({
        title: 'Lembrete Roubit',
        message: dataStr,
      })
    }
  }

  function scheduleReconnect(): void {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
    }

    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      if (authStore.isAuthenticated) {
        startListening()
      }
    }, retryDelayMs)

    // Exponential backoff
    retryDelayMs = Math.min(retryDelayMs * 2, MAX_RETRY_DELAY_MS)
  }

  function startListening(): void {
    if (eventSource || !authStore.token) {
      return
    }

    if (typeof EventSource === 'undefined') {
      return
    }

    const baseURL = (import.meta.env.VITE_API_BASE_URL || '/api/v1').replace(/\/$/, '')
    const streamUrl = `${baseURL}/notifications/stream?token=${encodeURIComponent(authStore.token)}`

    try {
      const es = new EventSource(streamUrl)
      eventSource = es

      es.onopen = () => {
        isConnected.value = true
        retryDelayMs = 1000
      }

      es.onmessage = (event: MessageEvent) => {
        if (event.data) {
          handleIncomingEvent(event.data)
        }
      }

      // Backend emits named "reminder" events
      es.addEventListener('reminder', (event: any) => {
        if (event.data) {
          handleIncomingEvent(event.data)
        }
      })

      es.onerror = () => {
        isConnected.value = false
        if (eventSource) {
          eventSource.close()
          eventSource = null
        }
        scheduleReconnect()
      }
    } catch {
      isConnected.value = false
      scheduleReconnect()
    }
  }

  function stopListening(): void {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }

    if (eventSource) {
      eventSource.close()
      eventSource = null
    }

    isConnected.value = false
    retryDelayMs = 1000
  }

  return {
    isConnected,
    unreadCount,
    recentNotifications,
    permissionState,
    addNotification,
    markAsRead,
    markAllAsRead,
    removeNotification,
    clearAll,
    requestSystemPermission,
    startListening,
    stopListening,
    disconnectSSE: stopListening,
  }
})
