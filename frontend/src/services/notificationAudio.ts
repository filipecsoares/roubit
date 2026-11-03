/**
 * Synthesizes a subtle, pleasant two-tone chime (520Hz -> 659Hz / C5 -> E5)
 * using the Web Audio API without requiring any external audio files.
 */

let audioCtx: AudioContext | null = null

function getAudioContext(): AudioContext | null {
  if (typeof window === 'undefined') return null
  const AudioContextClass =
    window.AudioContext ||
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext

  if (!AudioContextClass) return null
  if (!audioCtx) {
    audioCtx = new AudioContextClass()
  }
  return audioCtx
}

export function playNotificationChime(): void {
  try {
    const ctx = getAudioContext()
    if (!ctx) return

    if (ctx.state === 'suspended') {
      ctx.resume().catch(() => {})
    }

    const now = ctx.currentTime

    // Tone 1: 520Hz (C5 approx), duration 0.18s
    const osc1 = ctx.createOscillator()
    const gain1 = ctx.createGain()

    osc1.type = 'sine'
    osc1.frequency.setValueAtTime(520, now)

    gain1.gain.setValueAtTime(0.001, now)
    gain1.gain.exponentialRampToValueAtTime(0.12, now + 0.02)
    gain1.gain.exponentialRampToValueAtTime(0.0001, now + 0.18)

    osc1.connect(gain1)
    gain1.connect(ctx.destination)

    osc1.start(now)
    osc1.stop(now + 0.18)

    // Tone 2: 659Hz (E5), duration 0.35s, starts after 0.12s
    const osc2 = ctx.createOscillator()
    const gain2 = ctx.createGain()

    osc2.type = 'sine'
    osc2.frequency.setValueAtTime(659, now + 0.12)

    gain2.gain.setValueAtTime(0.001, now + 0.12)
    gain2.gain.exponentialRampToValueAtTime(0.15, now + 0.14)
    gain2.gain.exponentialRampToValueAtTime(0.0001, now + 0.45)

    osc2.connect(gain2)
    gain2.connect(ctx.destination)

    osc2.start(now + 0.12)
    osc2.stop(now + 0.45)
  } catch {
    // Graceful silent fallback if AudioContext is blocked or unsupported in browser
  }
}
