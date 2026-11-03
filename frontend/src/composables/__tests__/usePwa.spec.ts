import 'fake-indexeddb/auto'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { usePwa } from '../usePwa'

describe('usePwa Composable Test Suite (Activity 16.3 & 16.4)', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('detects online and offline events and updates isOnline reactively', () => {
    const { isOnline } = usePwa()

    // Trigger offline
    window.dispatchEvent(new Event('offline'))
    expect(isOnline.value).toBe(false)

    // Trigger online
    window.dispatchEvent(new Event('online'))
    expect(isOnline.value).toBe(true)
  })

  it('captures beforeinstallprompt and updates isInstallable', async () => {
    const { isInstallable, installApp } = usePwa()

    const mockPromptEvent = new Event('beforeinstallprompt') as any
    mockPromptEvent.prompt = vi.fn().mockResolvedValue(undefined)
    mockPromptEvent.userChoice = Promise.resolve({ outcome: 'accepted', platform: 'web' })

    window.dispatchEvent(mockPromptEvent)

    expect(isInstallable.value).toBe(true)

    const accepted = await installApp()
    expect(accepted).toBe(true)
    expect(mockPromptEvent.prompt).toHaveBeenCalled()
    expect(isInstallable.value).toBe(false)
  })

  it('interacts safely with App Badging API when supported', () => {
    const setAppBadgeMock = vi.fn().mockResolvedValue(undefined)
    const clearAppBadgeMock = vi.fn().mockResolvedValue(undefined)

    Object.defineProperty(navigator, 'setAppBadge', {
      value: setAppBadgeMock,
      writable: true,
      configurable: true,
    })
    Object.defineProperty(navigator, 'clearAppBadge', {
      value: clearAppBadgeMock,
      writable: true,
      configurable: true,
    })

    const { updateAppBadge, clearAppBadge } = usePwa()

    updateAppBadge(4)
    expect(setAppBadgeMock).toHaveBeenCalledWith(4)

    updateAppBadge(0)
    expect(clearAppBadgeMock).toHaveBeenCalled()

    clearAppBadge()
    expect(clearAppBadgeMock).toHaveBeenCalledTimes(2)
  })
})
