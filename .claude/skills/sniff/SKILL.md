---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, or adding new detection capabilities.
---

# sniff

Cross-platform system detection library and CLI for Rust.

## Capabilities

| Category | Detection |
|----------|-----------|
| OS | Distribution, kernel, architecture, package managers, locale, timezone |
| Hardware | CPU (with SIMD), GPU (Metal), memory, storage |
| Network | Interface enumeration with IPv4/IPv6 |
| Filesystem | Git repos, monorepos, languages, EditorConfig, document discovery |
| Programs | 8 categories with macOS bundle support |
| Services | 11 init systems (systemd, launchd, OpenRC, runit, etc.) |
| Packages | 110+ package manager abstraction |

## Quick Start

```rust
use sniff_lib::{detect, SniffConfig};

// Full detection
let result = detect()?;

// Configured detection
let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)           // Network queries for git/packages
    .commit_count(20)     // Recent commits (default: 10)
    .skip_network();      // Skip section

let result = detect_with_config(config)?;
```

## CLI

```bash
sniff                      # Full system info (JSON output)
sniff hardware             # Hardware only (text output)
sniff cpu                  # Just CPU info
sniff programs             # All programs
sniff editors              # Just editors
sniff agents               # AI CLI tools
sniff services             # System services
sniff docs                 # Markdown documents
sniff topics               # Table of available topics
sniff structure            # Structural overview
sniff hardware --json      # Subcommand with JSON output
```

**Output modes:**
- No subcommand: JSON (all data)
- With subcommand: Text (default), `--json` for JSON

**Programs JSON formats:**
- `sniff programs --json` - Simple format (backward compatible)
- `sniff programs --json --json-format full` - Rich metadata

## Key Types

| Type | Description |
|------|-------------|
| `SniffResult` | Top-level: os, hardware, network, filesystem |
| `SniffConfig` | Builder: base_dir, deep, commit_count, skip_* |
| `ProgramsInfo` | 8 category fields with parallel detection |
| `ServicesInfo` | Init system + service list |
| `Package` | Package path, languages, managers, dependencies |

## Detailed Topics

- [Programs](./programs.md) - 8 categories, macOS bundle detection
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference
