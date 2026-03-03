export type DeviceStatus = 'active' | 'standby' | 'error' | 'not_configured'
export type DotColor = 'green' | 'amber' | 'red' | 'grey'

export interface SonySource {
  category: string
  name: string
  active: boolean
  uri: string | null
}

export interface SonyDetail {
  volume?: number
  max_volume?: number
  muted?: boolean
  sources?: SonySource[]
}

export interface ArcamActiveDetail {
  model?: string
  muted?: boolean
  mode?: string
  auto_shutdown?: number
  auto_shutdown_label?: string
}

export interface ArcamStandbyDetail {
  model?: string
  heartbeat?: boolean
}

export interface DeviceStatusJson {
  status: DeviceStatus
  label: string
  detail?: SonyDetail | ArcamActiveDetail | ArcamStandbyDetail | null
}

export interface StatusResponse {
  sony: DeviceStatusJson
  arcam: DeviceStatusJson
}

export interface DeviceInfo {
  name: string
  host: string
  port: number
}

export const AUTO_SHUTDOWN_OPTIONS: Record<number, string> = {
  0: 'Off',
  1: '20 min',
  2: '30 min',
  3: '1 hour',
  4: '2 hours',
}

export type ServiceKind = 'sony_receiver' | 'arcam_amp' | 'eversolo' | 'samsung_tv'

export interface ServiceKindMeta {
  kind: ServiceKind
  label: string
  description: string
  defaultPort: number
  apiPath: string
}

export const SERVICE_KINDS: ServiceKindMeta[] = [
  { kind: 'sony_receiver', label: 'Sony Receiver', description: 'Sony ES audio receiver', defaultPort: 10000, apiPath: '/sony_receiver' },
  { kind: 'arcam_amp', label: 'Arcam Amplifier', description: 'Arcam PA-series amplifier', defaultPort: 50000, apiPath: '/arcam_amp' },
  { kind: 'eversolo', label: 'Eversolo Streamer', description: 'Eversolo DMP music streamer', defaultPort: 9529, apiPath: '/eversolo' },
  { kind: 'samsung_tv', label: 'Samsung TV', description: 'Samsung Smart TV', defaultPort: 8001, apiPath: '/samsung_tv' },
]

export interface CreateDeviceRequest {
  name: string
  host: string
  port?: number
}

export function statusToDotColor(status: DeviceStatus): DotColor {
  switch (status) {
    case 'active': return 'green'
    case 'standby': return 'amber'
    case 'error': return 'red'
    default: return 'grey'
  }
}
