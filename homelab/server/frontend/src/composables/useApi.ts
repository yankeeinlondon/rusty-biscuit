import type { CreateDeviceRequest, DeviceInfo, StatusResponse } from '~/types'

export function useApi() {
  async function getStatus(): Promise<StatusResponse> {
    const resp = await fetch('/status')
    if (!resp.ok) throw new Error(`status ${resp.status}`)
    return resp.json()
  }

  async function getDevices(): Promise<{ sony: DeviceInfo[], arcam: DeviceInfo[], eversolo: DeviceInfo[], samsung_tv: DeviceInfo[] }> {
    const [sonyResp, arcamResp, eversoloResp, samsungResp] = await Promise.all([
      fetch('/sony_receiver'),
      fetch('/arcam_amp'),
      fetch('/eversolo'),
      fetch('/samsung_tv'),
    ])
    const sony: DeviceInfo[] = sonyResp.ok ? await sonyResp.json() : []
    const arcam: DeviceInfo[] = arcamResp.ok ? await arcamResp.json() : []
    const eversolo: DeviceInfo[] = eversoloResp.ok ? await eversoloResp.json() : []
    const samsung_tv: DeviceInfo[] = samsungResp.ok ? await samsungResp.json() : []
    return { sony, arcam, eversolo, samsung_tv }
  }

  function toggleSonyPower(device: string, active: boolean) {
    fetch(`/sony_receiver/${encodeURIComponent(device)}/power`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ active }),
    }).catch(() => {})
  }

  function setSonyInput(device: string, uri: string) {
    fetch(`/sony_receiver/${encodeURIComponent(device)}/input`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ uri }),
    }).catch(() => {})
  }

  function toggleArcamPower(device: string, on: boolean) {
    const action = on ? 'on' : 'off'
    fetch(`/arcam_amp/${encodeURIComponent(device)}/power/${action}`, {
      method: 'POST',
    }).catch(() => {})
  }

  async function getAutoShutdown(): Promise<{ value: number, label: string, error?: string }> {
    const resp = await fetch('/arcam/auto-shutdown')
    return resp.json()
  }

  function setAutoShutdown(value: number) {
    fetch('/arcam/auto-shutdown', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ value }),
    }).catch(() => {})
  }

  async function getTimeout(): Promise<unknown> {
    const resp = await fetch('/arcam/timeout')
    if (!resp.ok) return null
    return resp.json()
  }

  function sendEversoloPower(device: string, tag: string) {
    fetch(`/eversolo/${encodeURIComponent(device)}/power`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tag }),
    }).catch(() => {})
  }

  function sendSamsungKey(device: string, key: string) {
    fetch(`/samsung_tv/${encodeURIComponent(device)}/remote/key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key }),
    }).catch(() => {})
  }

  function wakeEversolo(device: string) {
    fetch(`/eversolo/${encodeURIComponent(device)}/wake`, {
      method: 'PUT',
    }).catch(() => {})
  }

  function wakeSamsung(device: string) {
    fetch(`/samsung_tv/${encodeURIComponent(device)}/wake`, {
      method: 'PUT',
    }).catch(() => {})
  }

  async function createDevice(apiPath: string, body: CreateDeviceRequest): Promise<void> {
    const resp = await fetch(apiPath, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({ error: 'Request failed' }))
      throw new Error(err.error || `HTTP ${resp.status}`)
    }
  }

  return {
    getStatus,
    getDevices,
    createDevice,
    toggleSonyPower,
    setSonyInput,
    toggleArcamPower,
    getAutoShutdown,
    setAutoShutdown,
    getTimeout,
    sendEversoloPower,
    sendSamsungKey,
    wakeEversolo,
    wakeSamsung,
  }
}
