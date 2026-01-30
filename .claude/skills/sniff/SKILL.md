---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, or adding new detection capabilities.
---

# sniff

Cross-platform system detection library and CLI for Rust.

## Capabilities

| Category | Detection |
|----------|-----------|
| OS | Distribution, locale, timezone |
| Hardware | CPU, GPU, memory, storage |
| Network | Interface enumeration |
| Filesystem | Git repos, monorepos, languages |
| Programs | 8 categories with macOS bundle support |
| Services | Multiple init systems (systemd, launchd, etc.) |
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
    .skip_network();      // Skip section

let result = detect_with_config(config)?;
```

## CLI

```bash
sniff                      # Full system info
sniff --hardware           # Hardware only
sniff --programs           # All programs
sniff --editors            # Just editors
sniff --services           # System services
sniff --json               # JSON output
```

## Detailed Topics

- [Programs](./programs.md) - 8 categories, macOS bundle detection
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference
