import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useNotificationStore } from '../notification'
import { useAuthStore } from '../auth'

// Mock native browser EventSource
class MockEventSource {
  static instances: MockEventSource[] = []
  url: string
  onopen: ((event: any) => void) | null = null
  onmessage: ((event: any) => void) | null = null
  onerror: ((event: any) => void) | null = null
  listeners: Record<string, ((event: any) => void)[]> = {}
  readyState = 0
  closed = false

  constructor(url: string) {
    this.url = url
    MockEventSource.instances.push(this)
  }

  addEventListener(event: string, callback: (event: any) => void) {
    if (!this.listeners[event]) {
      this.listeners[event] = []
    }
    this.listeners[event].push(callback)
  }

  simulateOpen() {
    this.readyState = 1
    if (this.onopen) {
      this.onopen({ type: 'open' })
    }
  }

  simulateMessage(data: string) {
    const event = { data }
    if (this.onmessage) {
      this.onmessage(event)
    }
  }

  simulateCustomEvent(eventName: string, data: string) {
    const event = { data }
    if (this.listeners[eventName]) {
      for (const listener of this.listeners[eventName]) {
        listener(event)
      }
    }
  }

  simulateError() {
    this.readyState = 2
    if (this.onerror) {
      this.onerror({ type: 'error' })
    }
  }

  close() {
    this.closed = true
    this.readyState = 2
  }
}

// Mock native browser Notification API
class MockNotification {
  static instances: MockNotification[] = []
  static permission: NotificationPermission = 'default'
  static requestPermission = vi.fn().mockImplementation(async () => {
    MockNotification.permission = 'granted'
    return 'granted'
  })

  title: string
  options: any
  onclick: (() => void) | null = null
  close = vi.fn()

  constructor(title: string, options: any) {
    this.title = title
    this.options = options
    MockNotification.instances.push(this)
  }
}

describe('useNotificationStore (SSE Stream & Web Notifications)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
    MockEventSource.instances = []
    MockNotification.instances = []
    MockNotification.permission = 'default'

    // Stub browser globals
    vi.stubGlobal('EventSource', MockEventSource)
    vi.stubGlobal('Notification', MockNotification)
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('initializes with default clean state', () => {
    const store = useNotificationStore()

    expect(store.isConnected).toBe(false)
    expect(store.unreadCount).toBe(0)
    expect(store.recentNotifications).toHaveLength(0)
    expect(store.permissionState).toBe('default')
  })

  it('does not connect to SSE if user is not authenticated', () => {
    const authStore = useAuthStore()
    authStore.token = null

    const store = useNotificationStore()
    store.startListening()

    expect(MockEventSource.instances).toHaveLength(0)
    expect(store.isConnected).toBe(false)
  })

  it('establishes EventSource connection with auth token query param', () => {
    const authStore = useAuthStore()
    authStore.token = 'mock_jwt_token_123'

    const store = useNotificationStore()
    store.startListening()

    expect(MockEventSource.instances).toHaveLength(1)
    const es = MockEventSource.instances[0]
    expect(es.url).toContain('/notifications/stream?token=mock_jwt_token_123')
    expect(store.isConnected).toBe(false)

    es.simulateOpen()
    expect(store.isConnected).toBe(true)
  })

  it('receives reminder event and reactively updates recentNotifications and unreadCount', () => {
    const authStore = useAuthStore()
    authStore.token = 'mock_jwt_token_123'

    const store = useNotificationStore()
    store.startListening()

    const es = MockEventSource.instances[0]
    es.simulateOpen()

    const payload = JSON.stringify({
      user_id: 'usr_abc',
      entity_type: 'time_block',
      entity_id: 'blk_789',
      title: 'Rotina: Foco Profundo',
      message: 'O bloco está começando agora.',
      scheduled_at: '2026-11-02T21:00:00Z',
    })

    es.simulateCustomEvent('reminder', payload)

    expect(store.recentNotifications).toHaveLength(1)
    expect(store.unreadCount).toBe(1)
    expect(store.recentNotifications[0].title).toBe('Rotina: Foco Profundo')
    expect(store.recentNotifications[0].entity_type).toBe('time_block')
    expect(store.recentNotifications[0].read).toBe(false)
  })

  it('parses generic message events with plaintext fallback gracefully', () => {
    const authStore = useAuthStore()
    authStore.token = 'mock_token'

    const store = useNotificationStore()
    store.startListening()

    const es = MockEventSource.instances[0]
    es.simulateOpen()

    es.simulateMessage('Lembrete simples em texto puro')

    expect(store.recentNotifications).toHaveLength(1)
    expect(store.unreadCount).toBe(1)
    expect(store.recentNotifications[0].message).toBe('Lembrete simples em texto puro')
  })

  it('marks specific notification and all notifications as read', () => {
    const store = useNotificationStore()

    const n1 = store.addNotification({ title: 'Aviso 1', message: 'Msg 1' })
    const n2 = store.addNotification({ title: 'Aviso 2', message: 'Msg 2' })

    expect(store.unreadCount).toBe(2)

    store.markAsRead(n1.id)
    expect(store.unreadCount).toBe(1)
    expect(n1.read).toBe(true)
    expect(n2.read).toBe(false)

    store.markAllAsRead()
    expect(store.unreadCount).toBe(0)
    expect(n2.read).toBe(true)
  })

  it('removes notification and clears all notifications', () => {
    const store = useNotificationStore()

    const n1 = store.addNotification({ title: 'Aviso 1', message: 'Msg 1' })
    store.addNotification({ title: 'Aviso 2', message: 'Msg 2' })
    expect(store.recentNotifications).toHaveLength(2)

    store.removeNotification(n1.id)
    expect(store.recentNotifications).toHaveLength(1)
    expect(store.recentNotifications[0].title).toBe('Aviso 2')

    store.clearAll()
    expect(store.recentNotifications).toHaveLength(0)
    expect(store.unreadCount).toBe(0)
  })

  it('closes EventSource cleanly on stopListening', () => {
    const authStore = useAuthStore()
    authStore.token = 'mock_token'

    const store = useNotificationStore()
    store.startListening()

    const es = MockEventSource.instances[0]
    es.simulateOpen()
    expect(store.isConnected).toBe(true)

    store.stopListening()
    expect(es.closed).toBe(true)
    expect(store.isConnected).toBe(false)
  })

  it('reconnects with exponential backoff on connection error', () => {
    const authStore = useAuthStore()
    authStore.token = 'mock_token'

    const store = useNotificationStore()
    store.startListening()

    const es1 = MockEventSource.instances[0]
    es1.simulateOpen()
    expect(store.isConnected).toBe(true)

    // Trigger error -> connection closes, reconnect timer scheduled (1s)
    es1.simulateError()
    expect(store.isConnected).toBe(false)
    expect(es1.closed).toBe(true)

    // Advance 500ms -> should not have reconnected yet
    vi.advanceTimersByTime(500)
    expect(MockEventSource.instances).toHaveLength(1)

    // Advance 500ms more (total 1000ms) -> reconnects
    vi.advanceTimersByTime(500)
    expect(MockEventSource.instances).toHaveLength(2)

    const es2 = MockEventSource.instances[1]
    es2.simulateOpen()
    expect(store.isConnected).toBe(true)
  })

  it('dispatches native OS Notification when document is hidden and permission is granted', () => {
    // Simulate backgrounded browser tab
    Object.defineProperty(document, 'hidden', { value: true, configurable: true })

    const store = useNotificationStore()
    store.permissionState = 'granted'

    store.addNotification({
      title: 'Bloco Iniciado',
      message: 'Hora de produzir.',
      entity_type: 'time_block',
    })

    expect(MockNotification.instances).toHaveLength(1)
    const osNotif = MockNotification.instances[0]
    expect(osNotif.title).toBe('Bloco Iniciado')
    expect(osNotif.options.body).toBe('Hora de produzir.')

    // Simulate clicking the OS notification -> window focus and close
    const focusSpy = vi.spyOn(window, 'focus').mockImplementation(() => {})
    if (osNotif.onclick) {
      osNotif.onclick()
    }
    expect(focusSpy).toHaveBeenCalled()
    expect(osNotif.close).toHaveBeenCalled()

    Object.defineProperty(document, 'hidden', { value: false, configurable: true })
  })

  it('requests system permission via Notification.requestPermission()', async () => {
    const store = useNotificationStore()

    const permission = await store.requestSystemPermission()
    expect(MockNotification.requestPermission).toHaveBeenCalled()
    expect(permission).toBe('granted')
    expect(store.permissionState).toBe('granted')
  })
})
