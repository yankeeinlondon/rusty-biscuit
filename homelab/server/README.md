# homelab-server

REST API server for controlling home AV equipment via the homelab library.

## Overview

This server exposes REST endpoints for:

- **Sony Receiver** - Power, volume, mute, inputs, sources, zones, audio, IMAX, HDMI, network, system settings
- **Arcam Amplifier** - Power, mute, amplifier mode, temperature, auto-shutdown, display name

Designed for internal homelab networks (no authentication).

## Configuration

### Configuration File (Recommended)

The server uses `~/homey.json` for device configuration. This file is created automatically on first run.

```json
{
  "sony_receivers": {
    "living-room": { "host": "192.168.1.100", "port": 10000 },
    "bedroom": { "host": "192.168.1.101", "port": 10000 }
  },
  "arcam_amps": {
    "office": { "host": "192.168.1.102", "port": 50000 }
  }
}
```

### Environment Variables (Legacy)

For backward compatibility, you can still use environment variables. These are automatically migrated to the config file on first run.

| Variable | Description | Default |
|----------|-------------|---------|
| `SONY_RECEIVER` | Sony receiver host (format: `host` or `host:port`) | None |
| `ARCAM_AMP` | Arcam amplifier hostname or IP | None |
| `REQUEST_TIMEOUT_MS` | Request timeout in milliseconds | 5000 |
| `PORT` | Server listen port | 3000 |

> **Note:** The legacy `/sony/*` and `/arcam/*` routes are deprecated. Use the new `/sony_receiver/{name}/*` and `/arcam_amp/{name}/*` routes instead.

## Installation

```bash
cargo install --path homelab/server
```

Or via justfile:

```bash
just -f homelab/justfile install-server
```

## API Endpoints

### Health & Dashboard

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Live dashboard with SSE status updates |
| GET | `/status` | JSON status of all devices |
| GET | `/health` | Basic health check |
| GET | `/health/devices` | Device configuration status |
| GET | `/explore` | Scalar API documentation explorer |

### Sony Receiver Management

| Method | Path | Description |
|--------|------|-------------|
| GET | `/sony_receiver` | List all configured receivers |
| POST | `/sony_receiver` | Create a new receiver |
| PUT | `/sony_receiver/{name}` | Update a receiver |
| PATCH | `/sony_receiver/{name}` | Rename a receiver (plain text body) |
| DELETE | `/sony_receiver/{name}` | Delete a receiver |

### Sony Receiver Control

Basic controls using the Sony JSON-RPC API (port 10000).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/{name}/power` | Get power status |
| POST | `/{name}/power` | Set power (`{"active": bool}`) |
| GET | `/{name}/volume` | Get volume info |
| POST | `/{name}/volume` | Set volume (`{"level": 0-100}`) |
| GET | `/{name}/mute` | Get mute status |
| POST | `/{name}/mute` | Set mute (`{"mute": bool}`) |
| GET | `/{name}/inputs` | List available inputs (JSON-RPC terminal URIs) |
| GET | `/{name}/input/current` | Get current input |
| POST | `/{name}/input` | Set input by URI (`{"uri": "..."}`) |
| POST | `/{name}/source` | Set input by category name (`{"category": "GAME"}`) |
| GET | `/{name}/system/info` | Get system information (model, serial, MAC) |

> All Sony receiver paths are prefixed with `/sony_receiver`.

### Sony Receiver Zones

| Method | Path | Description |
|--------|------|-------------|
| GET | `/{name}/zone` | Main zone status (power, volume, mute, input) |
| GET | `/{name}/zone2` | Zone 2 status (power, volume, input) |
| GET | `/{name}/zone3` | Zone 3 status (power, volume, input) |

### Sony Receiver Native API

These endpoints use the Sony native HTTP API (`/fcgi-bin/request.fcgi` on port 80), which provides access to settings the JSON-RPC API doesn't expose. The native API works in both active and standby states.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/{name}/sources` | Input sources with user-defined names, HDMI assignments, visibility |
| GET | `/{name}/system/settings` | System settings (volume display, dimmer, device name, network) |
| GET | `/{name}/audio/settings` | Audio settings (sound field, pure direct, DSD, Bluetooth mode, etc.) |
| GET | `/{name}/audio/imax` | IMAX Enhanced config (crossovers, upmixer, subwoofer, mode) |
| GET | `/{name}/network` | Network config (IPv4/IPv6, DNS, connection type, WiFi) |
| GET | `/{name}/hdmi` | HDMI config (CEC, eARC, signal formats, source assignments) |

#### Native API Notes

The Sony STR-AZ7000ES exposes a native HTTP API on port 80 at `/fcgi-bin/request.fcgi` that is separate from the documented JSON-RPC API on port 10000.

Key differences from JSON-RPC:

- **Works in standby** - The native API responds even when the receiver is off (some features may return empty values)
- **User-defined source names** - The `inputname` feature reveals custom names (e.g. "PS5" for GAME, "AppleTV" for STB)
- **HDMI configuration** - Per-port signal formats, CEC, eARC, passthrough settings
- **IMAX Enhanced** - HPF crossover frequencies per speaker position, subwoofer settings
- **Packet format** - Requests use grouped feature arrays; max ~16 groups per request
- **Boolean values** - The native API uses `"on"`/`"off"` strings, not `"true"`/`"false"`
- **Unavailable features** - Return `"ERR"` or `"NAK"` (mapped to `null` in the API)

### Arcam Amplifier Management

| Method | Path | Description |
|--------|------|-------------|
| GET | `/arcam_amp` | List all configured amplifiers |
| POST | `/arcam_amp` | Create a new amplifier |
| PUT | `/arcam_amp/{name}` | Update an amplifier |
| PATCH | `/arcam_amp/{name}` | Rename an amplifier (plain text body) |
| DELETE | `/arcam_amp/{name}` | Delete an amplifier |

### Arcam Amplifier Control

| Method | Path | Description |
|--------|------|-------------|
| GET | `/{name}/status` | Full amplifier status |
| GET | `/{name}/power` | Get power state |
| POST | `/{name}/power/on` | Power on |
| POST | `/{name}/power/off` | Power off |
| GET | `/{name}/mute` | Get mute status |
| POST | `/{name}/mute/on` | Mute on |
| POST | `/{name}/mute/off` | Mute off |
| GET | `/{name}/mode` | Get amplifier mode |
| GET | `/{name}/temperature/{sensor_type}` | Get temperature reading |
| GET | `/{name}/timeout` | Get display timeout |
| GET | `/{name}/name` | Get display name |
| PUT | `/{name}/name` | Set display name |
| GET | `/{name}/auto-shutdown` | Get auto-shutdown setting |
| PUT | `/{name}/auto-shutdown` | Set auto-shutdown setting |

> All Arcam amplifier paths are prefixed with `/arcam_amp`.

### Legacy Routes (Deprecated)

These routes work when the corresponding environment variable is set:

| Method | Path | Description |
|--------|------|-------------|
| * | `/sony/*` | Legacy Sony routes (uses `SONY_RECEIVER` env) |
| * | `/arcam/*` | Legacy Arcam routes (uses `ARCAM_AMP` env) |

## Usage Examples

```bash
# Start the server
homelab-server

# Health check
curl http://localhost:3000/health

# List Sony receivers
curl http://localhost:3000/sony_receiver

# Add a Sony receiver (port defaults to 10000)
curl -X POST http://localhost:3000/sony_receiver \
  -H "Content-Type: application/json" \
  -d '{"name": "living-room", "host": "192.168.1.100"}'

# Get power status
curl http://localhost:3000/sony_receiver/living-room/power

# Set volume to 30
curl -X POST http://localhost:3000/sony_receiver/living-room/volume \
  -H "Content-Type: application/json" \
  -d '{"level": 30}'

# Mute receiver
curl -X POST http://localhost:3000/sony_receiver/living-room/mute \
  -H "Content-Type: application/json" \
  -d '{"mute": true}'

# Switch source by category
curl -X POST http://localhost:3000/sony_receiver/living-room/source \
  -H "Content-Type: application/json" \
  -d '{"category": "GAME"}'

# Get user-defined source names
curl http://localhost:3000/sony_receiver/living-room/sources

# Get HDMI configuration
curl http://localhost:3000/sony_receiver/living-room/hdmi

# Get network configuration
curl http://localhost:3000/sony_receiver/living-room/network

# Get IMAX Enhanced settings
curl http://localhost:3000/sony_receiver/living-room/audio/imax

# Power on Arcam
curl -X POST http://localhost:3000/arcam_amp/office/power/on

# Rename a device
curl -X PATCH http://localhost:3000/sony_receiver/living-room \
  -H "Content-Type: text/plain" \
  -d 'main-room'

# Delete a device
curl -X DELETE http://localhost:3000/sony_receiver/living-room
```

## Error Responses

All errors return JSON with `error` and `code` fields:

```json
{
  "error": "Device not found: living-room",
  "code": "DEVICE_NOT_FOUND"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `DEVICE_NOT_CONFIGURED` | 404 | Device not configured (legacy routes) |
| `DEVICE_NOT_FOUND` | 404 | Device name not in config |
| `DEVICE_EXISTS` | 409 | Device name already exists |
| `INVALID_DEVICE_NAME` | 400 | Invalid device name format |
| `INVALID_HOST` | 400 | Invalid or empty host |
| `SONY_ERROR` | 502 | Sony receiver communication error |
| `ARCAM_ERROR` | 502 | Arcam amplifier communication error |
| `INVALID_VOLUME` | 400 | Volume level out of range (0-100) |
| `TIMEOUT` | 504 | Device request timed out |
| `CONFIG_IO_ERROR` | 500 | Error reading/writing config file |
| `CONFIG_PARSE_ERROR` | 500 | Error parsing config file |

### Device Name Rules

Device names must be:
- Lowercase letters (a-z)
- Numbers (0-9)
- Underscores and hyphens only

Examples: `living-room`, `office_1`, `bedroom`

## API Documentation

Visit `/explore` when the server is running to access the Scalar API explorer.

## Development

```bash
# Run in development mode
just -f homelab/justfile server

# Run tests
just -f homelab/justfile test-server

# Build release
cargo build -p homelab-server --release
```
