import { describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import { usePolling } from './usePolling'

// Mock useDocumentVisibility to always return 'visible'
vi.mock('@vueuse/core', () => ({
  useDocumentVisibility: () => ({ value: 'visible' }),
}))

describe('usePolling', () => {
  it('calls fetcher immediately when immediate=true', async () => {
    const fetcher = vi.fn().mockResolvedValue({ ok: true })
    const scope = effectScope()

    scope.run(() => {
      usePolling(fetcher, { immediate: true, intervalMs: 10000 })
    })

    // Wait for async execution
    await vi.waitFor(() => expect(fetcher).toHaveBeenCalledTimes(1))

    scope.stop()
  })

  it('stores data from fetcher', async () => {
    const fetcher = vi.fn().mockResolvedValue({ value: 42 })
    const scope = effectScope()
    let polling: ReturnType<typeof usePolling<{ value: number }>>

    scope.run(() => {
      polling = usePolling(fetcher, { immediate: true, intervalMs: 10000 })
    })

    await vi.waitFor(() => expect(polling!.data.value).toEqual({ value: 42 }))

    scope.stop()
  })

  it('calls onSuccess callback', async () => {
    const onSuccess = vi.fn()
    const fetcher = vi.fn().mockResolvedValue('result')
    const scope = effectScope()

    scope.run(() => {
      usePolling(fetcher, { immediate: true, intervalMs: 10000, onSuccess })
    })

    await vi.waitFor(() => expect(onSuccess).toHaveBeenCalledWith('result'))

    scope.stop()
  })

  it('calls onError callback on failure', async () => {
    const onError = vi.fn()
    const error = new Error('fail')
    const fetcher = vi.fn().mockRejectedValue(error)
    const scope = effectScope()

    scope.run(() => {
      usePolling(fetcher, { immediate: true, intervalMs: 10000, onError })
    })

    await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(error))

    scope.stop()
  })

  it('polls at interval', async () => {
    vi.useFakeTimers()
    const fetcher = vi.fn().mockResolvedValue('ok')
    const scope = effectScope()

    scope.run(() => {
      usePolling(fetcher, { immediate: false, intervalMs: 1000 })
    })

    expect(fetcher).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1000)
    await vi.waitFor(() => expect(fetcher).toHaveBeenCalledTimes(1))

    vi.advanceTimersByTime(1000)
    await vi.waitFor(() => expect(fetcher).toHaveBeenCalledTimes(2))

    scope.stop()
    vi.useRealTimers()
  })

  it('stops polling on scope dispose', async () => {
    vi.useFakeTimers()
    const fetcher = vi.fn().mockResolvedValue('ok')
    const scope = effectScope()

    scope.run(() => {
      usePolling(fetcher, { immediate: false, intervalMs: 1000 })
    })

    scope.stop()

    vi.advanceTimersByTime(3000)
    expect(fetcher).not.toHaveBeenCalled()

    vi.useRealTimers()
  })
})
