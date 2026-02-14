# homey

CLI for controlling homelab AV equipment from the terminal.

## Installation

```bash
just -f homelab/justfile install
# or
cargo install --path homelab/cli
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SONY_RECEIVER` | Sony receiver IP or DNS name |
| `ARCAM_AMP` | Arcam amplifier IP or DNS name |

## Commands

### Completions

```bash
homey completions              # Show shell completions setup instructions
```

### Arcam

```bash
homey arcam on              # Power on
homey arcam off             # Power off
homey arcam power-status    # Get power state
homey arcam mute-status     # Get mute state
homey arcam mute-toggle     # Toggle mute
```

### Sony

Commands are grouped into subcommand categories:

**System** — Power, info, firmware updates

```bash
homey sony system power-status
homey sony system on / off
homey sony system info
homey sony system update-check
homey sony system update-apply         # Apply firmware update (reboots receiver)
homey sony system alexa-status         # Alexa registration status
homey sony system ecia-info            # ECIA device info
homey sony system wu-tang-info [name]  # WuTang provisioning info
```

**Audio** — Volume and mute control

```bash
homey sony audio volume
homey sony audio set-volume 30
homey sony audio mute-status
homey sony audio mute / unmute
homey sony audio speaker-settings [all|level|distance|size|pattern]
```

**Input** — Source selection, browsing, Bluetooth

```bash
homey sony input list
homey sony input current
homey sony input set "extInput:hdmi?port=1"
homey sony input schemes
homey sony input sources <scheme>
homey sony input content-count <source>
homey sony input content-list <source> [--start N] [--count N]  # defaults: 0, 100
homey sony input browse <source>
homey sony input set-terminal <uri>
homey sony input bluetooth [all|bt-standby|aac]
homey sony input set-bluetooth <target> <value>
homey sony input playback-mode [all|shuffle|repeat]
```

**Playback** — Transport controls

```bash
homey sony playback now-playing
homey sony playback stop / pause / next / previous
homey sony playback functions              # Available functions for current input
homey sony playback supported-functions    # All supported playback functions
homey sony playback preset <uri>           # Preset a broadcast station
homey sony playback seek <forward|backward>   # aliases: fwd, bwd
homey sony playback scan <forward|backward>   # aliases: fwd, bwd
```

**Debug** — API introspection

```bash
homey sony debug methods <endpoint>            # aliases: av, app, access
homey sony debug probe
```

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON instead of rich terminal rendering |

## Per-Command Flags

| Flag | Applies To | Description |
|------|-----------|-------------|
| `--host <IP>` | `arcam`, `sony` | Override device host (also set via env vars above) |
| `--port <PORT>` | `sony` | Override receiver port (default: 10000) |

## Shell Completions

```bash
# Bash
echo 'source <(COMPLETE=bash homey)' >> ~/.bashrc

# Zsh
echo 'source <(COMPLETE=zsh homey)' >> ~/.zshrc

# Fish
echo 'COMPLETE=fish homey | source' >> ~/.config/fish/config.fish
```

## Key Dependencies

- `biscuit-terminal` — Rich terminal rendering (tables, prose, lists)
- `clap` + `clap_complete` — CLI parsing with dynamic shell completions
- `color-eyre` — Error reporting
