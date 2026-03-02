import { ref, onScopeDispose } from 'vue'
import { useDocumentVisibility } from '@vueuse/core'

interface UsePollingOptions<T> {
  intervalMs?: number
  immediate?: boolean
  onSuccess?: (data: T) => void
  onError?: (error: unknown) => void
}

export function usePolling<T>(
  fetcher: () => Promise<T>,
  options: UsePollingOptions<T> = {},
) {
  const {
    intervalMs = 3000,
    immediate = true,
    onSuccess,
    onError,
  } = options

  const data = ref<T | null>(null) as { value: T | null }
  const error = ref<unknown>(null)
  let timer: ReturnType<typeof setInterval> | null = null
  const visibility = useDocumentVisibility()

  async function execute() {
    try {
      const result = await fetcher()
      data.value = result
      error.value = null
      onSuccess?.(result)
    }
    catch (e) {
      error.value = e
      onError?.(e)
    }
  }

  function start() {
    stop()
    timer = setInterval(() => {
      if (visibility.value === 'hidden') return
      execute()
    }, intervalMs)
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  if (immediate) execute()
  start()

  onScopeDispose(stop)

  return {
    data,
    error,
    execute,
    start,
    stop,
  }
}
