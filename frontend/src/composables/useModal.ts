import { watch, onUnmounted, type Ref } from 'vue'

let openModalCount = 0

/**
 * Composable to handle modal usability standards:
 * 1. Closes modal when user presses 'Escape'.
 * 2. Locks body scrolling while modal is open, restoring it on close (ref-counted).
 */
export function useModal(isOpen: Ref<boolean> | (() => boolean), onClose: () => void) {
  let wasOpen = false

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.code === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  function lockScroll() {
    if (typeof window === 'undefined' || typeof document === 'undefined') return
    if (!wasOpen) {
      wasOpen = true
      openModalCount++
      if (openModalCount === 1) {
        document.body.style.overflow = 'hidden'
      }
      window.addEventListener('keydown', handleKeyDown)
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
