import { watch, onUnmounted, nextTick, type Ref } from 'vue'

let openModalCount = 0

/**
 * Composable to handle modal usability and WAI-ARIA standards:
 * 1. Closes modal when user presses 'Escape'.
 * 2. Locks body scrolling while modal is open, restoring it on close (ref-counted).
 * 3. Traps keyboard focus (Tab / Shift+Tab) within containerRef when open.
 * 4. Restores focus to the trigger element that opened the modal upon closing.
 */
export function useModal(
  isOpen: Ref<boolean> | (() => boolean),
  onClose: () => void,
  containerRef?: Ref<HTMLElement | null>
) {
  let wasOpen = false
  let triggerElement: HTMLElement | null = null

  const FOCUSABLE_SELECTOR =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.code === 'Escape') {
      e.preventDefault()
      onClose()
      return
    }

    if (e.key === 'Tab' && containerRef?.value) {
      const focusables = Array.from(
        containerRef.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)
      ).filter((el) => el.offsetParent !== null || el === document.activeElement)

      if (focusables.length === 0) {
        e.preventDefault()
        return
      }

      const firstFocusable = focusables[0]
      const lastFocusable = focusables[focusables.length - 1]

      if (e.shiftKey) {
        if (document.activeElement === firstFocusable || !containerRef.value.contains(document.activeElement)) {
          e.preventDefault()
          lastFocusable.focus()
        }
      } else {
        if (document.activeElement === lastFocusable || !containerRef.value.contains(document.activeElement)) {
          e.preventDefault()
          firstFocusable.focus()
        }
      }
    }
  }

  function lockScroll() {
    if (typeof window === 'undefined' || typeof document === 'undefined') return
    if (!wasOpen) {
      wasOpen = true
      triggerElement = document.activeElement as HTMLElement | null
      openModalCount++
      if (openModalCount === 1) {
        document.body.style.overflow = 'hidden'
      }
      window.addEventListener('keydown', handleKeyDown)

      if (containerRef) {
        nextTick(() => {
          if (!containerRef.value) return
          const first = containerRef.value.querySelector<HTMLElement>(FOCUSABLE_SELECTOR)
          if (first && typeof first.focus === 'function') {
            first.focus()
          } else if (typeof containerRef.value.focus === 'function') {
            containerRef.value.focus()
          }
        })
      }
    }
  }

  function unlockScroll() {
    if (typeof window === 'undefined' || typeof document === 'undefined') return
    if (wasOpen) {
      wasOpen = false
      openModalCount = Math.max(0, openModalCount - 1)
      if (openModalCount === 0) {
        document.body.style.overflow = ''
      }
      window.removeEventListener('keydown', handleKeyDown)

      if (triggerElement && typeof triggerElement.focus === 'function') {
        const el = triggerElement
        triggerElement = null
        nextTick(() => {
          try {
            el.focus()
          } catch {
            // Ignored if element was unmounted
          }
        })
      }
    }
  }

  watch(
    typeof isOpen === 'function' ? isOpen : () => isOpen.value,
    (open) => {
      if (open) {
        lockScroll()
      } else {
        unlockScroll()
      }
    },
    { immediate: true, flush: 'sync' }
  )

  onUnmounted(() => {
    unlockScroll()
  })
}
