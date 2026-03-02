import { describe, expect, it, vi } from 'vitest'
import { useConnectionHealth } from './useConnectionHealth'

describe('useConnectionHealth', () => {
  it('starts reachable', () => {
    const health = useConnectionHealth()
    expect(health.isUnreachable.value).toBe(false)
  })

  it('stays reachable below threshold', () => {
    const health = useConnectionHealth(3)
    health.onFailure()
    health.onFailure()
    expect(health.isUnreachable.value).toBe(false)
  })

  it('becomes unreachable at threshold', () => {
    const health = useConnectionHealth(2)
    health.onFailure()
    health.onFailure()
    expect(health.isUnreachable.value).toBe(true)
  })

  it('reloads on recovery after being unreachable', () => {
    const reloadSpy = vi.fn()
    Object.defineProperty(window, 'location', {
      value: { reload: reloadSpy },
      writable: true,
    })

    const health = useConnectionHealth(1)
    health.onFailure()
    expect(health.isUnreachable.value).toBe(true)

    health.onSuccess()
    expect(reloadSpy).toHaveBeenCalledOnce()
  })

  it('resets failure count on success', () => {
    const health = useConnectionHealth(3)
    health.onFailure()
    health.onFailure()
    health.onSuccess()

    // After reset, need 3 more failures
    health.onFailure()
    health.onFailure()
    expect(health.isUnreachable.value).toBe(false)

    health.onFailure()
    expect(health.isUnreachable.value).toBe(true)
  })
})
