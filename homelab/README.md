# homelab

Home automation control for AV equipment over the local network.

## Packages

| Package | Binary | Description |
|---------|--------|-------------|
| `homelab` (lib) | — | Core library: Arcam amplifier + Sony ES receiver control |
| `homelab-cli` (cli) | `homey` | CLI for controlling AV devices from the terminal |
| `homelab-server` (server) | `homelab-server` | REST API server (Axum) for AV device control |

## Supported Devices

### Arcam
- PA240, PA410, PA720 amplifiers (binary protocol over TCP port 50000)

### Sony
- STR-ES, STR-DA, STR-ZA, STR-DN series receivers (JSON-RPC over HTTP port 10000)
- Native Web API (port 80) for zone status and settings

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SONY_RECEIVER` | Sony receiver IP or DNS name |
| `ARCAM_AMP` | Arcam amplifier IP or DNS name |

## Quick Start

```bash
# Build all packages
just -f homelab/justfile build

# Install the CLI
just -f homelab/justfile install

# Install the server
just -f homelab/justfile install-server

# Run the server in dev mode
just -f homelab/justfile server
```

## Configuration

Create `~/homey.json` to define your devices:

```json
{
  "sony_receivers": {
    "living-room": {
      "host": "192.168.1.100",
      "port": 10000
    }
  },
  "arcam_amps": {
    "office": {
      "host": "192.168.1.101"
    }
  }
}
```

Then use `--name` flag to select devices: `homey sony --name living-room system info`

## Key CLI Commands

```bash
# Arcam amplifier control
homey arcam on / off
homey arcam power-status
homey arcam mute-status / mute-toggle

# Sony receiver control
homey sony system on / off
homey sony system info
homey sony audio volume
homey sony audio set-volume 30
homey sony input list
homey sony input set "extInput:hdmi?port=1"
homey sony playback now-playing
homey sony native zone
```

## Key Dependencies

- `biscuit-terminal` — Rich terminal rendering (tables, prose, lists)
- `clap` + `clap_complete` — CLI parsing with dynamic shell completions
- `color-eyre` — Error reporting

## Sub-Package Documentation

- [Library](lib/README.md) — Module reference and Sony JSON-RPC lessons learned
- [CLI](cli/README.md) — Complete command reference, environment variables, shell completions
- [Server](server/README.md) — REST endpoints, error codes, usage examples
- [Sony STR-AZ7000ES](docs/sony-str-az7000es.md) — Network API reference for the receiver
