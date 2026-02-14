---
name: homelab
description: Home automation AV control library, CLI, and REST server for Sony ES receivers and Arcam amplifiers. Use when working in homelab/, controlling AV equipment, or building home automation features.
---

## Purpose

The `homelab` package provides local-network control of AV equipment:

- **Library** (`homelab/lib`) — Rust client library for Arcam amplifiers (TCP binary protocol) and Sony ES receivers (JSON-RPC over HTTP)
- **CLI** (`homelab-cli`) — Binary `homey` for terminal-based device control with rich rendering via `biscuit-terminal`
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
| `SONY_RECEIVER` | CLI, Server (legacy) | Sony receiver host (IP or DNS) |
| `ARCAM_AMP` | CLI, Server (legacy) | Arcam amplifier host (IP or DNS) |
| `PORT` | Server | Listen port (default: 3000) |
| `REQUEST_TIMEOUT_MS` | Server | Device timeout in ms (default: 5000) |

> Server uses `~/homey.json` config file for multi-device support. ENV vars are legacy and auto-migrated to the config file on first run.

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
homey (--json)
├── arcam (--host) [on|off|power-status|mute-status|mute-toggle]
├── sony (--host, --port)
│   ├── system [power-status|on|off|info|update-check|update-apply|
│   │           alexa-status|ecia-info|wu-tang-info]
│   ├── audio  [volume|set-volume|mute-status|mute|unmute|speaker-settings]
│   ├── input  [list|current|set|schemes|sources|content-count|content-list|
│   │           browse|set-terminal|bluetooth|set-bluetooth|playback-mode]
│   ├── playback [now-playing|stop|pause|next|previous|functions|
│   │             supported-functions|preset|seek|scan]
│   └── debug  [methods|probe]
└── completions
```

### REST API (Server)

Named device routes (`{name}` = device name from `~/homey.json`):

| Group | Endpoints |
|-------|-----------|
| Health | `GET /health`, `GET /health/devices` |
| Sony CRUD | `GET /sony_receiver`, `POST /sony_receiver`, `PUT /sony_receiver/{name}`, `PATCH /sony_receiver/{name}` (rename), `DELETE /sony_receiver/{name}` |
| Sony Control | `GET/POST /sony_receiver/{name}/power`, `GET/POST /sony_receiver/{name}/volume`, `GET/POST /sony_receiver/{name}/mute`, `GET /sony_receiver/{name}/inputs`, `GET /sony_receiver/{name}/input/current`, `POST /sony_receiver/{name}/input`, `GET /sony_receiver/{name}/system/info` |
| Arcam CRUD | `GET /arcam_amp`, `POST /arcam_amp`, `PUT /arcam_amp/{name}`, `PATCH /arcam_amp/{name}` (rename), `DELETE /arcam_amp/{name}` |
| Arcam Control | `GET /arcam_amp/{name}/power`, `POST /arcam_amp/{name}/power/on`, `POST /arcam_amp/{name}/power/off`, `GET /arcam_amp/{name}/mute`, `POST /arcam_amp/{name}/mute/on`, `POST /arcam_amp/{name}/mute/off`, `GET /arcam_amp/{name}/mode` |
| Legacy | `/sony/*`, `/arcam/*` (deprecated, uses ENV vars) |

> Interactive API explorer at `/explore` (Scalar UI).

## Key Dependencies

| Crate | Version | Used In |
|-------|---------|---------|
| `reqwest` | 0.13.2 | lib (Sony HTTP) |
| `axum` | 0.8 | server |
| `tower` / `tower-http` | 0.5 / 0.6 | server middleware |
| `clap` + `clap_complete` | 4.5 | cli |
| `color-eyre` | 0.6 | cli (error reporting) |
| `biscuit-terminal` | local | cli (rich rendering) |
| `tokio` | 1.48-1.49 | all |

## Architectural Patterns

- **Sony receiver** uses `serde_json::Value` for all response parsing due to API inconsistencies (see [sony-quirks.md](sony-quirks.md))
- **Arcam amplifier** uses a binary protocol over TCP port 50000
- **Server** uses `~/homey.json` for multi-device config with named devices; legacy ENV vars auto-migrate on first run
- **Server state** stores devices in `Arc<RwLock<HashMap<String, Device>>>` for concurrent access; CRUD ops auto-save to disk
- **CLI** uses dual output: rich `biscuit-terminal` rendering or `--json` for machine consumption
- **CLI flags**: `--json` is global; `--host` and `--port` are per-command (on `arcam` and `sony` subcommands)
- **All device operations** in the server are wrapped with configurable timeouts

## Detailed References

- [Sony JSON-RPC Quirks](sony-quirks.md) — Response format gotchas, ghost methods, field inconsistencies
- [Architecture](architecture.md) — Library, CLI, and server design details
