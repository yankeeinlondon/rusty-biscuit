import { ref, onMounted } from 'vue'
import type {
  ArcamActiveDetail,
  ArcamStandbyDetail,
  DeviceInfo,
  DeviceStatusJson,
  DotColor,
  SonyDetail,
  StatusResponse,
} from '~/types'
import { AUTO_SHUTDOWN_OPTIONS, statusToDotColor } from '~/types'
import { useApi } from './useApi'
import { useConnectionHealth } from './useConnectionHealth'
import { useOptimisticLock } from './useOptimisticLock'
import { usePolling } from './usePolling'

export function useDeviceControl() {
  const api = useApi()
  const health = useConnectionHealth()

  // Device info
  const sonyDeviceInfo = ref<DeviceInfo | null>(null)
  const arcamDeviceInfo = ref<DeviceInfo | null>(null)

  // Status state
  const sonyStatus = ref<DeviceStatusJson>({ status: 'not_configured', label: 'Loading...' })
  const arcamStatus = ref<DeviceStatusJson>({ status: 'not_configured', label: 'Loading...' })

  // Cached details for optimistic rendering
  const lastSonyDetail = ref<SonyDetail | null>(null)
  const lastArcamOnDetail = ref<ArcamActiveDetail | null>(null)
  const lastArcamModel = ref<string | null>(null)

  // Optimistic locks
  const sonyPowerLock = useOptimisticLock<DotColor>(15000)
  const arcamPowerLock = useOptimisticLock<DotColor>(15000)
  const sonySourceLock = useOptimisticLock<string>(10000)
  const arcamAutoShutdownLock = useOptimisticLock<number>(15000)

  function handleStatusUpdate(data: StatusResponse) {
    health.onSuccess()

    // Cache details
    if (data.sony.detail && 'sources' in (data.sony.detail as SonyDetail)) {
      lastSonyDetail.value = data.sony.detail as SonyDetail
    }
    if (data.arcam.detail && (data.arcam.detail as ArcamStandbyDetail).model) {
      lastArcamModel.value = (data.arcam.detail as ArcamStandbyDetail).model!
    }
    if (data.arcam.status === 'active' && data.arcam.detail) {
      lastArcamOnDetail.value = data.arcam.detail as ArcamActiveDetail
    }

    // Sony power lock
    if (sonyPowerLock.isActive()) {
      const serverColor = statusToDotColor(data.sony.status)
      if (serverColor === sonyPowerLock.lockedValue.value) {
        sonyPowerLock.confirm()
      }
      else {
        // Still transitioning — keep optimistic state, don't update
        // But still process source lock below
        processSourceLock(data)
        return
      }
    }

    // Process source lock
    processSourceLock(data)

    // Update Sony status
    sonyStatus.value = data.sony

    // Arcam power lock
    if (arcamPowerLock.isActive()) {
      const serverColor = statusToDotColor(data.arcam.status)
      if (serverColor === arcamPowerLock.lockedValue.value) {
        arcamPowerLock.confirm()
      }
      else {
        return
      }
    }

    // Auto-shutdown lock
    if (arcamAutoShutdownLock.isActive() && data.arcam.status === 'active' && data.arcam.detail) {
      const detail = data.arcam.detail as ArcamActiveDetail
      if (detail.auto_shutdown === arcamAutoShutdownLock.lockedValue.value) {
        arcamAutoShutdownLock.confirm()
      }
      else {
        detail.auto_shutdown = arcamAutoShutdownLock.lockedValue.value!
        detail.auto_shutdown_label = AUTO_SHUTDOWN_OPTIONS[arcamAutoShutdownLock.lockedValue.value!] || 'Unknown'
      }
    }
    else {
      arcamAutoShutdownLock.confirm()
    }

    arcamStatus.value = data.arcam
  }

  function processSourceLock(data: StatusResponse) {
    if (sonySourceLock.isActive() && data.sony.detail) {
      const detail = data.sony.detail as SonyDetail
      if (detail.sources) {
        const confirmed = detail.sources.some(
          s => s.category === sonySourceLock.lockedValue.value && s.active,
        )
        if (confirmed) {
          sonySourceLock.confirm()
        }
        else {
          detail.sources.forEach((s) => {
            s.active = s.category === sonySourceLock.lockedValue.value
          })
        }
      }
    }
    else {
      sonySourceLock.confirm()
    }
  }

  // Actions
  function toggleSonyPower() {
    const currentColor = statusToDotColor(sonyStatus.value.status)
    if (currentColor === 'grey') return

    const isOn = currentColor === 'green'
    const expectedColor: DotColor = isOn ? 'amber' : 'green'

    sonyPowerLock.set(expectedColor)

    // Optimistic UI update
    sonyStatus.value = {
      ...sonyStatus.value,
      status: isOn ? 'standby' : 'active',
      label: isOn ? 'Standby' : 'Active',
      detail: isOn ? null : lastSonyDetail.value,
    }

    const device = sonyDeviceInfo.value?.name
    if (device) api.toggleSonyPower(device, !isOn)
  }

  function toggleArcamPower() {
    const currentColor = statusToDotColor(arcamStatus.value.status)
    if (currentColor === 'grey') return

    const isOn = currentColor === 'green'
    const expectedColor: DotColor = isOn ? 'amber' : 'green'

    arcamPowerLock.set(expectedColor)

    // Optimistic UI update
    if (isOn) {
      arcamStatus.value = {
        ...arcamStatus.value,
        status: 'standby',
        label: 'Standby',
        detail: lastArcamModel.value ? { model: lastArcamModel.value } : null,
      }
    }
    else {
      arcamStatus.value = {
        ...arcamStatus.value,
        status: 'active',
        label: 'Active',
        detail: lastArcamOnDetail.value,
      }
    }

    const device = arcamDeviceInfo.value?.name
    if (device) api.toggleArcamPower(device, !isOn)
  }

  function selectSonySource(category: string, uri: string) {
    sonySourceLock.set(category)

    // Optimistic update
    if (lastSonyDetail.value?.sources) {
      lastSonyDetail.value.sources.forEach((s) => {
        s.active = s.category === category
      })
    }

    const currentDetail = sonyStatus.value.detail as SonyDetail | undefined
    if (currentDetail?.sources) {
      currentDetail.sources.forEach((s) => {
        s.active = s.category === category
      })
      sonyStatus.value = { ...sonyStatus.value }
    }

    const device = sonyDeviceInfo.value?.name
    if (device) api.setSonyInput(device, uri)
  }

  function setAutoShutdown(value: number) {
    arcamAutoShutdownLock.set(value)

    // Optimistic update
    const label = AUTO_SHUTDOWN_OPTIONS[value] || 'Unknown'
    if (lastArcamOnDetail.value) {
      lastArcamOnDetail.value.auto_shutdown = value
      lastArcamOnDetail.value.auto_shutdown_label = label
    }

    const detail = arcamStatus.value.detail as ArcamActiveDetail | undefined
    if (detail) {
      detail.auto_shutdown = value
      detail.auto_shutdown_label = label
      arcamStatus.value = { ...arcamStatus.value }
    }

    api.setAutoShutdown(value)
  }

  onMounted(async () => {
    // Fetch device names
    try {
      const devices = await api.getDevices()
      if (devices.sony.length > 0) sonyDeviceInfo.value = devices.sony[0]
      if (devices.arcam.length > 0) arcamDeviceInfo.value = devices.arcam[0]
    }
    catch {
      // Devices may not be configured yet
    }

    // Start status polling
    usePolling(api.getStatus, {
      intervalMs: 3000,
      immediate: true,
      onSuccess: handleStatusUpdate,
      onError: () => health.onFailure(),
    })

    // Start timeout polling (debug only)
    usePolling(api.getTimeout, {
      intervalMs: 30000,
      immediate: true,
      onSuccess: (data) => {
        if (data) console.log('[arcam] timeout_counter raw:', data)
      },
    })
  })

  return {
    // State
    sonyStatus,
    arcamStatus,
    lastSonyDetail,
    lastArcamOnDetail,
    lastArcamModel,
    isUnreachable: health.isUnreachable,
    sonyDeviceInfo,
    arcamDeviceInfo,
    // Locks (for checking in template)
    sonyPowerLock,
    arcamPowerLock,
    // Actions
    toggleSonyPower,
    toggleArcamPower,
    selectSonySource,
    setAutoShutdown,
  }
}
