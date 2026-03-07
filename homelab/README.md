# homelab

Home automation control for AV equipment over the local network.

## Packages

| Package | Binary | Description |
|---------|--------|-------------|
| `homelab` | — | Core library: Arcam amplifier + Sony ES receiver control |
| `homelab-cli` | `homey` | CLI for controlling AV devices from the terminal |
| `homelab-server` | `homelab-server` | REST API server (Axum) for AV device control |
| `arcam-amp-integration` | `arcam-amp-integration` | Unfolded Circle external integration for Arcam amplifiers |
| `sony-receiver-integration` | `sony-receiver-integration` | Unfolded Circle external integration for Sony ES receivers |
| `eversolo-integration` | `eversolo-integration` | Unfolded Circle external integration for Eversolo streamers |

## Supported AV Devices

- **Arcam Amplifiers** (_supports PA240, PA410, PA720 though focus has been on PA240_)
    - Discrete on/off endpoints
    - Provides a _heartbeat_ service which can keep the Arcam from deep sleep (making it inaccessible)
- **Sony AZ7000ES AV Receiver** (_likely to work on other AZ models_)
    - Discrete on/off endpoints
    - Enumerates input sources (factory and user defined)
    - Allows selection of discrete input source
    - Allows for discrete mute on/off
    - 
- **Samsung SmartTV** (_S95C is focus_)
    - Discrete off, Wake on LAN for on
    - 
- **Eversolo Streamer** (_A8 is focus_)
    - Allows enumeration of input sources
    - Allows setting discrete input source
    - Discrete power off
    - Uses WOL packet for power on
    - 


## Supported Homelab Services (future)

- MQTT Subscriber
- Ping Monitor
- 

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
- [Eversolo Integration](eversolo-integration/README.md) — UC Remote external integration for Eversolo
- [Sony STR-AZ7000ES](docs/sony-str-az7000es.md) — Network API reference for the receiver
