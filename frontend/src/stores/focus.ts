import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FocusModeType, FocusState, FocusSession } from '@/types'

export const POMODORO_FOCUS_SECONDS = 25 * 60      // 25 minutes
export const POMODORO_SHORT_BREAK_SECONDS = 5 * 60  // 5 minutes
export const POMODORO_LONG_BREAK_SECONDS = 15 * 60  // 15 minutes

export const useFocusStore = defineStore('focus', () => {
  const mode = ref<FocusModeType>('pomodoro')
  const durationSeconds = ref(POMODORO_FOCUS_SECONDS)
  const remainingSeconds = ref(POMODORO_FOCUS_SECONDS)
  const state = ref<FocusState>('idle')
  const completedCycles = ref(0)
  const associatedBlockId = ref<string | undefined>(undefined)
  const associatedBlockName = ref<string | undefined>(undefined)

  const isModalOpen = ref(false)

  let intervalId: ReturnType<typeof setInterval> | null = null
  let targetEndTime = 0

  const session = computed<FocusSession>(() => ({
    mode: mode.value,
    durationSeconds: durationSeconds.value,
    remainingSeconds: remainingSeconds.value,
    state: state.value,
    associatedBlockId: associatedBlockId.value,
    associatedBlockName: associatedBlockName.value,
    completedCycles: completedCycles.value,
  }))

  const isRunning = computed(() => state.value === 'running' || state.value === 'break')
  const isBreak = computed(() => mode.value === 'short_break' || mode.value === 'long_break')

  const progressPercentage = computed(() => {
    if (durationSeconds.value <= 0) return 0
    const elapsed = durationSeconds.value - remainingSeconds.value
    return Math.min(100, Math.max(0, Math.round((elapsed / durationSeconds.value) * 100)))
  })

  const formattedTime = computed(() => {
    const mins = Math.floor(remainingSeconds.value / 60)
    const secs = remainingSeconds.value % 60
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
  })

  /**
   * Synthesize a gentle notification chime using native Web Audio API (Zero external assets)
   */
  function playChime() {
    try {
      const AudioCtx = window.AudioContext || (window as any).webkitAudioContext
      if (!AudioCtx) return

      const ctx = new AudioCtx()
      if (ctx.state === 'suspended') {
        ctx.resume()
      }

      const now = ctx.currentTime
      // Harmonic chime chord (523.25Hz C5 -> 659.25Hz E5 -> 783.99Hz G5)
      const freqs = [523.25, 659.25, 783.99]

      freqs.forEach((freq, idx) => {
        const osc = ctx.createOscillator()
        const gain = ctx.createGain()

        osc.type = 'sine'
        osc.frequency.setValueAtTime(freq, now + idx * 0.08)

        gain.gain.setValueAtTime(0.001, now + idx * 0.08)
        gain.gain.exponentialRampToValueAtTime(0.18, now + idx * 0.08 + 0.03)
        gain.gain.exponentialRampToValueAtTime(0.0001, now + idx * 0.08 + 1.2)

        osc.connect(gain)
        gain.connect(ctx.destination)

        osc.start(now + idx * 0.08)
        osc.stop(now + idx * 0.08 + 1.3)
      })

      // Mobile haptic feedback if supported
      if (typeof navigator !== 'undefined' && 'vibrate' in navigator) {
        navigator.vibrate([100, 50, 100])
      }
    } catch {
      // Audio autoplay policy or headless test environment without Web Audio
    }
  }

  function tick() {
    const now = Date.now()
    const diff = Math.max(0, Math.round((targetEndTime - now) / 1000))
    remainingSeconds.value = diff

    if (diff <= 0) {
      handleSessionComplete()
    }
  }

  function handleSessionComplete() {
    stopInterval()
    remainingSeconds.value = 0
    state.value = 'completed'

    playChime()

    if (mode.value === 'pomodoro' || mode.value === 'block') {
      completedCycles.value += 1
    }
  }

  function stopInterval() {
    if (intervalId) {
      clearInterval(intervalId)
      intervalId = null
    }
  }

  function startFocus(
    newMode: FocusModeType = 'pomodoro',
    customSeconds?: number,
    block?: { id: string; name: string }
  ) {
    stopInterval()

    mode.value = newMode
    if (block) {
      associatedBlockId.value = block.id
      associatedBlockName.value = block.name
    }

    if (customSeconds && customSeconds > 0) {
      durationSeconds.value = customSeconds
    } else if (newMode === 'short_break') {
      durationSeconds.value = POMODORO_SHORT_BREAK_SECONDS
    } else if (newMode === 'long_break') {
      durationSeconds.value = POMODORO_LONG_BREAK_SECONDS
    } else {
      durationSeconds.value = POMODORO_FOCUS_SECONDS
    }

    remainingSeconds.value = durationSeconds.value
    state.value = (newMode === 'short_break' || newMode === 'long_break') ? 'break' : 'running'

    targetEndTime = Date.now() + remainingSeconds.value * 1000
    intervalId = setInterval(tick, 500)
  }

  function pauseTimer() {
    if (state.value !== 'running' && state.value !== 'break') return
    stopInterval()
    // Snap to exact remaining seconds
    const now = Date.now()
    remainingSeconds.value = Math.max(0, Math.round((targetEndTime - now) / 1000))
    state.value = 'paused'
  }

  function resumeTimer() {
    if (state.value !== 'paused') return
    state.value = isBreak.value ? 'break' : 'running'
    targetEndTime = Date.now() + remainingSeconds.value * 1000
    intervalId = setInterval(tick, 500)
  }

  function togglePlayPause() {
    if (state.value === 'running' || state.value === 'break') {
      pauseTimer()
    } else if (state.value === 'paused') {
      resumeTimer()
    } else if (state.value === 'idle' || state.value === 'completed') {
      startFocus(mode.value, durationSeconds.value)
    }
  }

  function resetTimer() {
    stopInterval()
    remainingSeconds.value = durationSeconds.value
    state.value = 'idle'
  }

  function skipToBreak() {
    const isLong = completedCycles.value > 0 && completedCycles.value % 4 === 0
    startFocus(isLong ? 'long_break' : 'short_break')
  }

  function skipToFocus() {
    startFocus('pomodoro')
  }

  function openFocusModal(block?: { id: string; name: string; start_time?: string; end_time?: string }) {
    if (block) {
      associatedBlockId.value = block.id
      associatedBlockName.value = block.name

      // If timer is idle, setup focus for this block
      if (state.value === 'idle' || state.value === 'completed') {
        startFocus('pomodoro', POMODORO_FOCUS_SECONDS, block)
      }
    }
    isModalOpen.value = true
  }

  function closeFocusModal() {
    isModalOpen.value = false
  }

  return {
    mode,
    durationSeconds,
    remainingSeconds,
    state,
    completedCycles,
    associatedBlockId,
    associatedBlockName,
    isModalOpen,
    session,
    isRunning,
    isBreak,
    progressPercentage,
    formattedTime,
    startFocus,
    pauseTimer,
    resumeTimer,
    togglePlayPause,
    resetTimer,
    skipToBreak,
    skipToFocus,
    openFocusModal,
    closeFocusModal,
    playChime,
  }
})
