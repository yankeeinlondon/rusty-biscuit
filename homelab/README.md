# homelab

Home automation control for AV equipment over the local network.

## Packages

| Package | Binary | Description |
|---------|--------|-------------|
| `homelab` (lib) | — | Core library: Arcam amplifier + Sony ES receiver control |
| `homelab-cli` (cli) | `homey` | CLI for controlling AV devices from the terminal |
| `homelab-server` (server) | `homelab-server` | REST API server (Axum) for AV device control |

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

## Sub-Package Documentation

- [Library](lib/README.md) — Module reference and Sony JSON-RPC lessons learned
- [CLI](cli/README.md) — Command reference, environment variables, shell completions
- [Server](server/README.md) — REST endpoints, error codes, usage examples
- [Sony STR-AZ7000ES](docs/sony-str-az7000es.md) — Network API reference for the receiver
