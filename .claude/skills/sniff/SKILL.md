---
name: sniff
description: Expert knowledge for sniff-lib and sniff-cli, a cross-platform system detection library and CLI for Rust. Use when detecting OS/hardware/network/filesystem info, program detection, service detection, adding new detection capabilities, or optimizing detection performance.
---

# sniff

Cross-platform system detection library and CLI for Rust.

## Capabilities

| Category | Detection |
|----------|-----------|
| OS | Distribution, kernel, architecture, package managers, locale, timezone |
| Hardware | CPU (with SIMD), GPU (Metal), memory, storage, audio devices |
| Network | Interface enumeration with IPv4/IPv6, WAN IP |
| Filesystem | Git repos, monorepos, languages, EditorConfig, document discovery |
| Programs | 8 categories with macOS bundle support (parallel via Rayon) |
| Services | 11 init systems (systemd, launchd, OpenRC, runit, etc.) |
| Packages | 110+ package manager abstraction |

## API Tiers

Three levels of API, from simplest to most flexible:

### Tier 1: Convenience (full defaults)

```rust
use sniff::detect;
let result = detect()?;
```

### Tier 2: Plan-based (fine-grained control)

```rust
use sniff::{detect_with_plan, request::*};

let plan = DetectionPlan::new()
    .os(OsRequest::summary())           // Core identity only
    .hardware(HardwareRequest::summary()) // CPU + memory only
    .without_network()                   // Skip entirely
    .filesystem(
        FilesystemRequest::new()
            .git(GitRequest::summary())  // Branch + dirty counts
            .repo(RepoRequest::structure()) // Workspace tools only
            .without_docs()
    );
let result = detect_with_plan(plan)?;
```

### Tier 3: Module-level (expert composition)

```rust
use sniff::filesystem::git::GitRepo;
use sniff::hardware::detect_hardware_summary;
use sniff::os::detect_os_with_request;

// Pick exactly what you need
let git = GitRepo::discover(path)?;
let hw = detect_hardware_summary()?;
```

### Legacy: SniffConfig (still supported)

```rust
use sniff::{detect_with_config, SniffConfig};
let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)           // Equivalent to GitRequest::deep()
    .commit_count(20)
    .skip_network();
let result = detect_with_config(config)?;
```

## Request Types

| Type | Presets | Controls |
|------|---------|----------|
| `OsRequest` | `summary()`, `full()` | Package managers, locale, NTP/time |
| `HardwareRequest` | `summary()`, `full()` | Storage, GPU, audio (~1.5s macOS) |
| `NetworkRequest` | `interfaces_only()`, `full()` | WAN IP lookup (HTTP) |
| `GitRequest` | `summary()`, `full()`, `deep()` | Commits, file changes, worktrees, remote refresh |
| `RepoRequest` | `structure()`, `full()` | Per-package language scanning |
| `FilesystemRequest` | `new()` (default full) | Composes git + repo + inventory + formatting + docs |
| `DetectionPlan` | `new()` (default full) | Composes all domains |

## Key Types

| Type | Description |
|------|-------------|
| `SniffResult` | Top-level: os, hardware, network, filesystem (all Optional) |
| `DetectionPlan` | Plan-based config with per-domain request types |
| `ProgramsInfo` | 8 category fields with parallel detection via Rayon |
| `ServicesInfo` | Init system + service list |
| `Package` | Package path, languages, managers, dependencies |

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

## Detailed Topics

- [Programs](./programs.md) - 8 categories, macOS bundle detection
- [Services](./services.md) - Init systems, service listing
- [Extending](./extending.md) - Add new detection capabilities
- [Architecture](../../../sniff/docs/sniff-library-architecture.md) - Cost model, shared-work design

## Resources

- [CLI README](../../../sniff/cli/README.md) - Complete CLI usage
- [Library README](../../../sniff/lib/README.md) - API reference
