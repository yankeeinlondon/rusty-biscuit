---
name: homelab
description: Home automation AV control library, CLI, and REST server for Sony ES receivers and Arcam amplifiers. Use when working in homelab/, controlling AV equipment, or building home automation features.
---

## Purpose

The `homelab` package provides local-network control of AV equipment:

- **Library** (`homelab/lib`) — Rust client library for Arcam amplifiers (TCP binary protocol) and Sony ES receivers (JSON-RPC over HTTP)
- **CLI** (`homelab/cli`) — Binary `homey` for terminal-based device control with rich rendering via `biscuit-terminal`
- **Server** (`homelab/server`) — Binary `homelab-server` (Axum REST API) exposing device control over HTTP

## Package Structure

```
homelab/
├── lib/          # Core library (arcam, network, sony_receiver modules)
├── cli/          # Binary: homey
├── server/       # Binary: homelab-server (Axum)
├── docs/         # Sony STR-AZ7000ES API reference
└── justfile      # DevOps commands (build, test, lint, install, server)
```

## Quick Reference

### Environment Variables

| Variable | Used By | Description |
|----------|---------|-------------|
| `SONY_RECEIVER` | CLI | Sony receiver host (IP or DNS) |
| `ARCAM_AMP` | CLI | Arcam amplifier host (IP or DNS) |
| `SONY_RECEIVER_HOST` | Server | Sony host (format: `host` or `host:port`) |
| `ARCAM_HOST` | Server | Arcam host |
| `PORT` | Server | Listen port (default: 3000) |
| `REQUEST_TIMEOUT_MS` | Server | Device timeout in ms (default: 5000) |

### Common Commands

```bash
just -f homelab/justfile build          # Build lib + cli + server
just -f homelab/justfile test           # Test all packages
just -f homelab/justfile lint           # Clippy on all packages
just -f homelab/justfile install        # Install homey CLI
just -f homelab/justfile install-server # Install server binary
just -f homelab/justfile server         # Run server in dev mode
```

### CLI Command Tree

```
homey
├── arcam [on|off|power-status|mute-status|mute-toggle]
├── sony
│   ├── system [power-status|on|off|info|update-check|...]
│   ├── audio  [volume|set-volume|mute|unmute|speaker-settings|...]
│   ├── input  [list|current|set|schemes|sources|bluetooth|...]
│   ├── playback [now-playing|stop|pause|next|previous|...]
│   └── debug  [methods|probe]
└── completions
```

### REST API (Server)

| Group | Endpoints |
|-------|-----------|
| Health | `GET /health`, `GET /health/devices` |
| Sony | `GET/POST /sony/power`, `GET/POST /sony/volume`, `GET/POST /sony/mute`, `GET /sony/inputs`, `GET /sony/input/current`, `POST /sony/input`, `GET /sony/system/info` |
| Arcam | `GET /arcam/power`, `POST /arcam/power/on`, `POST /arcam/power/off`, `GET /arcam/mute`, `POST /arcam/mute/on`, `POST /arcam/mute/off`, `GET /arcam/mode` |

## Key Dependencies

| Crate | Version | Used In |
|-------|---------|---------|
| `reqwest` | 0.13.2 | lib (Sony HTTP) |
| `axum` | 0.8 | server |
| `tower` / `tower-http` | 0.5 / 0.6 | server middleware |
| `clap` + `clap_complete` | 4.5 | cli |
| `biscuit-terminal` | local | cli (rich rendering) |
| `tokio` | 1.48-1.49 | all |

## Architectural Patterns

- **Sony receiver** uses `serde_json::Value` for all response parsing due to API inconsistencies (see [sony-quirks.md](sony-quirks.md))
- **Arcam amplifier** uses a binary protocol over TCP port 50000
- **Server** pre-creates a shared `Arc<SonyReceiver>` but creates Arcam clients per-request
- **CLI** uses dual output: rich `biscuit-terminal` rendering or `--json` for machine consumption
- **All device operations** in the server are wrapped with configurable timeouts

## Detailed References

- [Sony JSON-RPC Quirks](sony-quirks.md) — Response format gotchas, ghost methods, field inconsistencies
- [Architecture](architecture.md) — Library, CLI, and server design details
