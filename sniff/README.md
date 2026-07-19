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
  <li>Software inventory with install support (editors, utilities, package managers, test runners, etc.)</li>
  <li>System services and init system detection</li>
  <li>Git repository analysis, monorepo detection, and remote inspection</li>
  <li>Blast radius analysis, recent commits, and justfile detection</li>
  <li>Cross-platform (macOS, Linux, Windows)</li>
</ul>
</td>
</tr>
</table>

## Modules

### 1. Sniff Library (`sniff/lib`)

A comprehensive Rust library for system detection:

- **OS Detection**: Distribution, kernel, architecture, package managers, locale, timezone, NTP status
- **Hardware Detection**: CPU (with SIMD), GPU (Metal support), memory, storage, audio devices
- **Network Detection**: Interface enumeration with IPv4/IPv6 addresses plus WAN IP lookup (TTL-cached)
- **Filesystem Analysis**: Git repos, monorepo tools, language detection, file type classification, EditorConfig, document discovery, blast radius, justfile detection, recent commits
- **Programs Module**: Detect installed software across 10 categories, with installation support for 8, test-runner availability, and remote-bash consent gating
- **Services Module**: Detect and list system services across 10+ init systems
- **Remote Inspection**: Query GitHub, GitLab, Gitea, and Bitbucket APIs for repository metadata

The library exposes three API tiers: `detect()` for convenience, `detect_with_plan(DetectionPlan)` for fine-grained control, and module-level functions for expert composition. See [sniff/lib/README.md](lib/README.md) for the full API and [sniff/docs/sniff-library-architecture.md](docs/sniff-library-architecture.md) for the cost model and shared-work design.

### 2. Sniff CLI (`sniff/cli`)

A feature-rich CLI exposing all library capabilities:

- **Flexible Output**: Text (with verbosity levels), JSON, or plain text
- **Subcommand Filtering**: Use subcommands to show specific sections
- **Scoped Enrichment**: `--refresh-remotes` for git sync, `--latest-versions` for dependency updates
- **Software Detection**: List programs across 10 categories; install across the 8 that support installation
- **Repository Queries**: Recent commits, source code changes, blast radius analysis

See [sniff/cli/README.md](cli/README.md) for complete CLI documentation.

## Quick Start

```bash
# Install
cargo install --path sniff/cli

# Show help (no subcommand)
sniff

# Full system info as JSON
sniff --json

# Show only hardware (text output)
sniff hardware

# Show only CPU info
sniff cpu

# Show installed software
sniff software

# Show system services
sniff services

# Show audio devices
sniff audio-devices

# Detect justfiles and recipes
sniff just

# Repository analysis
sniff repo                           # Repository name
sniff repo recent-commits 1w         # Commits from last week
sniff repo git-status                # Git status with commit history
sniff repo branches                  # Local branches from known refs
sniff repo package-dependencies      # Internal workspace dependency graph
sniff repo dependencies              # External package dependencies
sniff repo git-status --compact      # Status section only
sniff repo remote origin             # Inspect remote repository
sniff blast-radius                   # Docs affected by dirty changes

# Scoped enrichment (opt-in network queries)
sniff repo structure --latest-versions  # Check registries for updates
sniff repo git-status --refresh-remotes  # Sync remote tracking data

# JSON output for subcommands
sniff hardware --json
```

## Detection Categories

| Category | Description |
|----------|-------------|
| **OS** | Distribution, kernel, architecture, package managers, locale, timezone, NTP |
| **Hardware** | CPU (SIMD), GPU (Metal), memory, storage, audio devices |
| **Network** | Interfaces, IPv4/IPv6 addresses, WAN IP |
| **Filesystem** | Git repos, monorepos, languages, file types, docs, blast radius, justfiles |
| **Programs** | Editors, utilities, package managers, TTS, terminals, audio, AI tools, test runners |
| **Services** | System services across 10+ init systems |
| **Remote** | GitHub, GitLab, Gitea, Bitbucket repository metadata |

## Project Structure

```
sniff/
├── cli/              # Binary crate (`sniff` command)
│   ├── src/
│   │   ├── main.rs       # Entry point, tracing initialization
│   │   ├── args.rs       # Clap subcommands and argument parsing
│   │   ├── commands.rs   # Command execution logic
│   │   ├── install.rs    # Program installation interface
│   │   └── output/       # Text/JSON rendering with per-topic modules
│   │       ├── mod.rs, filesystem.rs, hardware.rs, os.rs, network.rs
│   │       ├── programs.rs, services.rs, remote.rs, recent_commits.rs
│   │       └── topics.rs, just.rs
│   └── Cargo.toml
├── lib/              # Library crate
│   ├── src/
│   │   ├── lib.rs            # Public API: detect(), SniffConfig, DetectionPlan
│   │   ├── request.rs        # Fine-grained detection control types
│   │   ├── error.rs          # Error types
│   │   ├── os/               # OS detection (distro, locale, time, package managers)
│   │   ├── hardware/         # CPU, GPU, memory, storage, audio devices
│   │   ├── network/          # Network interfaces
│   │   ├── filesystem/       # Git, repo, languages, docs, blast radius, file types, just
│   │   ├── package/          # Package manager abstraction (110+)
│   │   ├── programs/         # Program detection (10 categories; 8 installable)
│   │   ├── remote/           # Remote repo inspection (GitHub, GitLab, Gitea, Bitbucket)
│   │   └── services/         # System service detection (10+ init systems)
│   └── Cargo.toml
└── docs/             # Package documentation
    └── sniff-library-architecture.md   # API tiers, cost model, shared-work design
```

## Further Reading

- [sniff/docs/sniff-library-architecture.md](docs/sniff-library-architecture.md) -- cost profile per subsection, shared-work strategies, and common caller profiles

## License

Part of the Rusty Biscuit monorepo. See top-level LICENSE file.
