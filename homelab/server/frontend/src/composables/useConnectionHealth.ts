import { ref } from 'vue'

export function useConnectionHealth(threshold = 2) {
  const isUnreachable = ref(false)
  let consecutiveFailures = 0

  function onSuccess() {
    if (isUnreachable.value) {
      window.location.reload()
      return
    }
    consecutiveFailures = 0
  }

  function onFailure() {
    consecutiveFailures++
    if (consecutiveFailures >= threshold && !isUnreachable.value) {
      isUnreachable.value = true
    }
  }

  return {
    isUnreachable,
    onSuccess,
    onFailure,
  }
}
