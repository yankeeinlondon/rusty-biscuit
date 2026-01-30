# Sniff

> System Sniffer (lib and CLI)

<table>
<tr>
<td><img src="../assets/sniff-512.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Sniff</h2>
<p>A comprehensive cross-platform system and repository detection tool.</p>

<ul>
  <li>OS, hardware, network, filesystem detection</li>
  <li>Program inventory (editors, utilities, package managers, etc.)</li>
  <li>System services and init system detection</li>
  <li>Git repository and monorepo analysis</li>
  <li>Cross-platform with automatic provider failover</li>
</ul>
</td>
</tr>
</table>

## Modules

### 1. Sniff Library (`sniff/lib`)

A comprehensive Rust library for system detection:

- **OS Detection**: Distribution, kernel, architecture, package managers, locale, timezone
- **Hardware Detection**: CPU (with SIMD), GPU (Metal support), memory, storage
- **Network Detection**: Interface enumeration with IPv4/IPv6 addresses
- **Filesystem Analysis**: Git repos, monorepo tools, language detection, EditorConfig
- **Programs Module**: Detect installed programs across 8 categories
- **Services Module**: Detect and list system services across init systems

See [sniff/lib/README.md](lib/README.md) for detailed API documentation.

### 2. Sniff CLI (`sniff/cli`)

A feature-rich CLI exposing all library capabilities:

- **Flexible Output**: Text (with verbosity levels) or JSON
- **Section Filtering**: Include-only mode, skip mode, and detail filters
- **Deep Mode**: Network queries for remote git branches and package versions
- **Program Detection**: List installed editors, utilities, package managers, and more

See [sniff/cli/README.md](cli/README.md) for complete CLI documentation.

## Quick Start

```bash
# Install
cargo install --path sniff/cli

# Detect everything
sniff

# JSON output
sniff --json

# Show only hardware
sniff --hardware

# Show installed programs
sniff --programs

# Show system services
sniff --services

# Deep mode (queries remotes and registries)
sniff --deep
```

## Detection Categories

| Category | Description |
|----------|-------------|
| **OS** | Distribution, kernel, architecture, package managers |
| **Hardware** | CPU, GPU, memory, storage |
| **Network** | Interfaces, IPv4/IPv6 addresses |
| **Filesystem** | Git repos, monorepos, languages |
| **Programs** | Editors, utilities, package managers, TTS, terminals, AI tools |
| **Services** | System services with init system detection |

## Project Structure

```
sniff/
├── cli/              # Binary crate (`sniff` command)
│   ├── src/
│   │   ├── main.rs   # CLI parsing, config, enrichment
│   │   └── output.rs # Text/JSON rendering with filtering
│   └── Cargo.toml
└── lib/              # Library crate
    ├── src/
    │   ├── lib.rs                    # Public API, SniffConfig
    │   ├── os.rs                     # OS detection
    │   ├── hardware.rs               # CPU, GPU, memory, storage
    │   ├── network.rs                # Network interfaces
    │   ├── filesystem/               # Git, repo, languages
    │   ├── package/                  # Package manager abstraction
    │   ├── programs/                 # Program detection (8 categories)
    │   └── services/                 # System service detection
    └── Cargo.toml
```

## License

Part of the Dockhand monorepo. See top-level LICENSE file.
