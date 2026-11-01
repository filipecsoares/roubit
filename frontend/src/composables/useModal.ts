import { watch, onUnmounted, type Ref } from 'vue'

/**
 * Composable to handle modal usability standards:
 * 1. Closes modal when user presses 'Escape'.
 * 2. Locks body scrolling while modal is open, restoring it on close.
 */
export function useModal(isOpen: Ref<boolean> | (() => boolean), onClose: () => void) {
  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' || e.code === 'Escape') {
      e.preventDefault()
      onClose()
    }
  }

  watch(
    typeof isOpen === 'function' ? isOpen : () => isOpen.value,
    (open) => {
      if (typeof window === 'undefined' || typeof document === 'undefined') return
      if (open) {
        document.body.style.overflow = 'hidden'
        window.addEventListener('keydown', handleKeyDown)
      } else {
        document.body.style.overflow = ''
        window.removeEventListener('keydown', handleKeyDown)
      }
    },
    { immediate: true }
  )

  onUnmounted(() => {
    if (typeof window !== 'undefined' && typeof document !== 'undefined') {
      document.body.style.overflow = ''
      window.removeEventListener('keydown', handleKeyDown)
    }
  })
}
