import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useFocusStore, POMODORO_FOCUS_SECONDS, POMODORO_SHORT_BREAK_SECONDS, POMODORO_LONG_BREAK_SECONDS } from '../focus'

describe('useFocusStore (Pomodoro & Focus Mode Engine)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it('initializes with default 25m pomodoro in idle state', () => {
    const store = useFocusStore()

    expect(store.mode).toBe('pomodoro')
    expect(store.durationSeconds).toBe(POMODORO_FOCUS_SECONDS)
    expect(store.remainingSeconds).toBe(POMODORO_FOCUS_SECONDS)
    expect(store.state).toBe('idle')
    expect(store.completedCycles).toBe(0)
    expect(store.formattedTime).toBe('25:00')
    expect(store.progressPercentage).toBe(0)
  })

  it('starts focus session and accurately ticks countdown', () => {
    const store = useFocusStore()

    store.startFocus('pomodoro', 60) // 1 minute test session
    expect(store.state).toBe('running')
    expect(store.remainingSeconds).toBe(60)

    // Advance 30 seconds
    vi.advanceTimersByTime(30000)
    expect(store.remainingSeconds).toBe(30)
    expect(store.formattedTime).toBe('00:30')
    expect(store.progressPercentage).toBe(50)
  })

  it('pauses and resumes timer gracefully', () => {
    const store = useFocusStore()

    store.startFocus('pomodoro', 100)
    vi.advanceTimersByTime(20000)
    expect(store.remainingSeconds).toBe(80)

    store.pauseTimer()
    expect(store.state).toBe('paused')

    // Advancing while paused does NOT decrease remaining seconds
    vi.advanceTimersByTime(30000)
    expect(store.remainingSeconds).toBe(80)

    store.resumeTimer()
    expect(store.state).toBe('running')
    vi.advanceTimersByTime(10000)
    expect(store.remainingSeconds).toBe(70)
  })

  it('resets timer to original duration', () => {
    const store = useFocusStore()

    store.startFocus('pomodoro', 120)
    vi.advanceTimersByTime(40000)
    expect(store.remainingSeconds).toBe(80)

    store.resetTimer()
    expect(store.state).toBe('idle')
    expect(store.remainingSeconds).toBe(120)
  })

  it('transitions to completed and increments cycles when countdown expires', () => {
    const store = useFocusStore()

    store.startFocus('pomodoro', 10)
    expect(store.completedCycles).toBe(0)

    vi.advanceTimersByTime(10000)
    expect(store.remainingSeconds).toBe(0)
    expect(store.state).toBe('completed')
    expect(store.completedCycles).toBe(1)
  })

  it('switches between focus and short/long breaks correctly', () => {
    const store = useFocusStore()

    // Skip to break when cycles = 0 -> short break (5m)
    store.skipToBreak()
    expect(store.mode).toBe('short_break')
    expect(store.durationSeconds).toBe(POMODORO_SHORT_BREAK_SECONDS)
    expect(store.state).toBe('break')

    // Simulate 4 completed pomodoros -> next break is long break (15m)
    store.completedCycles = 4
    store.skipToBreak()
    expect(store.mode).toBe('long_break')
    expect(store.durationSeconds).toBe(POMODORO_LONG_BREAK_SECONDS)
    expect(store.state).toBe('break')

    // Skip back to focus
    store.skipToFocus()
    expect(store.mode).toBe('pomodoro')
    expect(store.durationSeconds).toBe(POMODORO_FOCUS_SECONDS)
    expect(store.state).toBe('running')
  })

  it('opens and closes focus modal with associated time block context', () => {
    const store = useFocusStore()

    expect(store.isModalOpen).toBe(false)
    store.openFocusModal({ id: 'block-focus-1', name: 'Deep Coding' })

    expect(store.isModalOpen).toBe(true)
    expect(store.associatedBlockId).toBe('block-focus-1')
    expect(store.associatedBlockName).toBe('Deep Coding')
    expect(store.state).toBe('running')

    store.closeFocusModal()
    expect(store.isModalOpen).toBe(false)
  })
})
