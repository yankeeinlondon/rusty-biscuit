import { describe, expect, it, vi } from 'vitest'
import { useOptimisticLock } from './useOptimisticLock'

describe('useOptimisticLock', () => {
  it('starts unlocked', () => {
    const lock = useOptimisticLock()
    expect(lock.isLocked.value).toBe(false)
    expect(lock.lockedValue.value).toBeNull()
  })

  it('locks with a value', () => {
    const lock = useOptimisticLock<string>()
    lock.set('green')
    expect(lock.isLocked.value).toBe(true)
    expect(lock.lockedValue.value).toBe('green')
  })

  it('confirms and releases lock', () => {
    const lock = useOptimisticLock<string>()
    lock.set('amber')
    lock.confirm()
    expect(lock.isLocked.value).toBe(false)
    expect(lock.lockedValue.value).toBeNull()
  })

  it('checkAndConfirm releases when server matches', () => {
    const lock = useOptimisticLock<string>()
    lock.set('green')
    const result = lock.checkAndConfirm('green')
    expect(result).toBe(true)
    expect(lock.isLocked.value).toBe(false)
  })

  it('checkAndConfirm keeps lock when server differs', () => {
    const lock = useOptimisticLock<string>()
    lock.set('green')
    const result = lock.checkAndConfirm('amber')
    expect(result).toBe(false)
    expect(lock.isLocked.value).toBe(true)
  })

  it('expires after duration', () => {
    vi.useFakeTimers()
    const lock = useOptimisticLock<string>(1000)
    lock.set('green')

    vi.advanceTimersByTime(999)
    expect(lock.isActive()).toBe(true)

    vi.advanceTimersByTime(2)
    expect(lock.isActive()).toBe(false)
    expect(lock.isLocked.value).toBe(false)

    vi.useRealTimers()
  })

  it('checkAndConfirm auto-releases on expiry', () => {
    vi.useFakeTimers()
    const lock = useOptimisticLock<string>(500)
    lock.set('green')

    vi.advanceTimersByTime(501)
    const result = lock.checkAndConfirm('amber')
    expect(result).toBe(false)
    expect(lock.isLocked.value).toBe(false)

    vi.useRealTimers()
  })
})
