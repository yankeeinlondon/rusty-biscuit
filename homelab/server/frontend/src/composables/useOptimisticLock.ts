import { ref } from 'vue'

export function useOptimisticLock<T>(durationMs = 15000) {
  const isLocked = ref(false)
  const lockedValue = ref<T | null>(null) as { value: T | null }
  let expiry = 0

  function set(value: T) {
    lockedValue.value = value
    isLocked.value = true
    expiry = Date.now() + durationMs
  }

  function confirm() {
    lockedValue.value = null
    isLocked.value = false
    expiry = 0
  }

  /** Check if server value matches locked value; if so, confirm and release lock */
  function checkAndConfirm(serverValue: T): boolean {
    if (!isLocked.value) return false
    if (Date.now() >= expiry) {
      confirm()
      return false
    }
    if (serverValue === lockedValue.value) {
      confirm()
      return true
    }
    return false
  }

  /** Returns true if lock is active and not expired */
  function isActive(): boolean {
    if (!isLocked.value) return false
    if (Date.now() >= expiry) {
      confirm()
      return false
    }
    return true
  }

  return {
    isLocked,
    lockedValue,
    set,
    confirm,
    checkAndConfirm,
    isActive,
  }
}
