# homelab-server

REST API server for controlling home AV equipment via the homelab library.

## Overview

This server exposes REST endpoints for:

- **Sony Receiver** - Power, volume, mute, inputs, system info
- **Arcam Amplifier** - Power, mute, amplifier mode

Designed for internal homelab networks (no authentication).

## Installation

```bash
cargo install --path homelab/server
```

Or via justfile:

```bash
just -f homelab/justfile install-server
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SONY_RECEIVER` | Sony receiver host (format: `host` or `host:port`) | None |
| `ARCAM_AMP` | Arcam amplifier hostname or IP | None |
| `REQUEST_TIMEOUT_MS` | Request timeout in milliseconds | 5000 |
| `PORT` | Server listen port | 3000 |

### Host Format Examples

```bash
# IPv4
SONY_RECEIVER=192.168.1.100

# IPv4 with custom port
SONY_RECEIVER=192.168.1.100:8080

# DNS name
SONY_RECEIVER=receiver.local

# IPv6
SONY_RECEIVER=[::1]
```

## API Endpoints

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Basic health check |
| GET | `/health/devices` | Device configuration status |

### Sony Receiver

| Method | Path | Description |
|--------|------|-------------|
| GET | `/sony/power` | Get power status |
| POST | `/sony/power` | Set power (`{"active": bool}`) |
| GET | `/sony/volume` | Get volume info |
| POST | `/sony/volume` | Set volume (`{"level": 0-100}`) |
| GET | `/sony/mute` | Get mute status |
| POST | `/sony/mute` | Set mute (`{"mute": bool}`) |
| GET | `/sony/inputs` | List available inputs |
| GET | `/sony/input/current` | Get current input |
| POST | `/sony/input` | Set input (`{"uri": "..."}`) |
| GET | `/sony/system/info` | Get system information |

### Arcam Amplifier

| Method | Path | Description |
|--------|------|-------------|
| GET | `/arcam/power` | Get power state |
| POST | `/arcam/power/on` | Power on |
| POST | `/arcam/power/off` | Power off |
| GET | `/arcam/mute` | Get mute status |
| POST | `/arcam/mute/on` | Mute on |
| POST | `/arcam/mute/off` | Mute off |
| GET | `/arcam/mode` | Get amplifier mode |

## Usage Examples

```bash
# Start the server
SONY_RECEIVER=192.168.1.100 homelab-server

# Health check
curl http://localhost:3000/health

# Check device configuration
curl http://localhost:3000/health/devices

# Get Sony power status
curl http://localhost:3000/sony/power

# Set volume to 30
curl -X POST http://localhost:3000/sony/volume \
  -H "Content-Type: application/json" \
  -d '{"level": 30}'

# Mute Sony receiver
curl -X POST http://localhost:3000/sony/mute \
  -H "Content-Type: application/json" \
  -d '{"mute": true}'

# Power on Arcam
curl -X POST http://localhost:3000/arcam/power/on
```

## Error Responses

All errors return JSON with `error` and `code` fields:

```json
{
  "error": "Sony Receiver not configured",
  "code": "DEVICE_NOT_CONFIGURED"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `DEVICE_NOT_CONFIGURED` | 404 | Device env var not set |
| `SONY_ERROR` | 502 | Sony receiver communication error |
| `ARCAM_ERROR` | 502 | Arcam amplifier communication error |
| `INVALID_VOLUME` | 400 | Volume level out of range (0-100) |
| `TIMEOUT` | 504 | Device request timed out |

## Development

```bash
# Run in development mode
just -f homelab/justfile server

# Run tests
just -f homelab/justfile test-server

# Build release
cargo build -p homelab-server --release
```
