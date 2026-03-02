import type { DeviceInfo, StatusResponse } from '~/types'

export function useApi() {
  async function getStatus(): Promise<StatusResponse> {
    const resp = await fetch('/status')
    if (!resp.ok) throw new Error(`status ${resp.status}`)
    return resp.json()
  }

  async function getDevices(): Promise<{ sony: DeviceInfo[], arcam: DeviceInfo[] }> {
    const [sonyResp, arcamResp] = await Promise.all([
      fetch('/sony_receiver'),
      fetch('/arcam_amp'),
    ])
    const sony: DeviceInfo[] = sonyResp.ok ? await sonyResp.json() : []
    const arcam: DeviceInfo[] = arcamResp.ok ? await arcamResp.json() : []
    return { sony, arcam }
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

  return {
    getStatus,
    getDevices,
    toggleSonyPower,
    setSonyInput,
    toggleArcamPower,
    getAutoShutdown,
    setAutoShutdown,
    getTimeout,
  }
}
