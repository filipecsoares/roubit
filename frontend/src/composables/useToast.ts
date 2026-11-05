import { ref, onUnmounted } from 'vue'

export interface ToastState {
  text: string
  type: 'success' | 'error'
}

/**
 * Extracts a human-readable error message from unknown errors,
 * giving highest precedence to RFC 7807 problem details (detail/title) from Axum/Axios.
 */
export function extractErrorMessage(err: unknown, fallback = 'Ocorreu um erro inesperado'): string {
  if (!err) return fallback
  if (typeof err === 'string') return err.trim() || fallback

  const anyErr = err as any
  if (anyErr.response?.data) {
    const data = anyErr.response.data
    if (typeof data.detail === 'string' && data.detail.trim().length > 0) {
      return data.detail.trim()
    }
    if (typeof data.title === 'string' && data.title.trim().length > 0) {
      return data.title.trim()
    }
    if (typeof data.message === 'string' && data.message.trim().length > 0) {
      return data.message.trim()
    }
    if (typeof data === 'string' && data.trim().length > 0) {
      return data.trim()
    }
  }

  if (typeof anyErr.message === 'string' && anyErr.message.trim().length > 0) {
    return anyErr.message.trim()
  }

  return fallback
}

/**
 * Composable providing isolated toast notification state with timer deduplication,
 * RFC 7807 error extraction, and automatic unmount cleanup.
 */
export function useToast() {
  const toastMessage = ref<ToastState | null>(null)
  let timerId: ReturnType<typeof setTimeout> | null = null

  function clearToast() {
    if (timerId !== null) {
      clearTimeout(timerId)
      timerId = null
    }
    toastMessage.value = null
  }

  function showToast(content: unknown, type: 'success' | 'error' = 'success', durationMs = 4000) {
    if (timerId !== null) {
      clearTimeout(timerId)
      timerId = null
    }

    const text = type === 'error' ? extractErrorMessage(content) : (typeof content === 'string' ? content : String(content))
    toastMessage.value = { text, type }

    timerId = setTimeout(() => {
      toastMessage.value = null
      timerId = null
    }, durationMs)
  }

  onUnmounted(() => {
    if (timerId !== null) {
      clearTimeout(timerId)
      timerId = null
    }
  })

  return {
    toastMessage,
    showToast,
    clearToast,
  }
}
