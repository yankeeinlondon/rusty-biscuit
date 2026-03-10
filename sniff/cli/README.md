# Sniff CLI

**Sniff** is a comprehensive system and repository detection tool that provides detailed information about your operating system, hardware, network, and filesystem environment. It's designed to give developers, system administrators, and automation tools a complete snapshot of the execution context.

## Features

- **OS Detection**: Distribution, kernel, architecture, hostname, package managers, locale, timezone, and NTP status
- **Hardware Detection**: CPU (with SIMD capabilities), GPU (with Metal/Vulkan support), memory, and storage
- **Network Detection**: Network interfaces with IPv4/IPv6 addresses, status flags, and WAN IP lookup
- **Filesystem Detection**: Git repository status, monorepo detection, programming language analysis, broad file associations, and EditorConfig formatting rules
- **Scoped Enrichment**: Refresh git remotes with `--refresh-remotes` and check registries with `--latest-versions`
- **Flexible Output**: Text (with verbosity levels) or JSON formats

## Installation

```bash
# Install from workspace root
just install

# Or install directly
cargo install --path sniff/cli
```

## Usage

### Basic Usage

```bash
# Detect everything (JSON output, no subcommand)
sniff

# Detect with a specific base directory
sniff --base /path/to/project

# Enable verbose output (show more details)
sniff -v        # Level 1: more details
sniff -vv       # Level 2: even more details
```

### Output Modes

The CLI has two output modes depending on whether a subcommand is used:

| Mode | Output | Use Case |
|------|--------|----------|
| No subcommand (`sniff`) | JSON (all data) | Programmatic consumption, piping to `jq` |
| With subcommand (`sniff cpu`) | Text (default) | Human-readable output |

```bash
# Full system info as JSON (no subcommand)
sniff

# Subcommand with text output (default)
sniff cpu

# Subcommand with JSON output
sniff cpu --json
```

### Section Subcommands

Use subcommands to filter output to specific sections.

**Top-Level Sections:**

```bash
sniff os          # OS information (name, kernel, locale, timezone)
sniff hardware    # Hardware information (CPU, GPU, memory, storage)
sniff network     # Network information (interfaces, local IPs, WAN IP)
sniff filesystem  # Filesystem information (git, languages, monorepo)
```

**Discovery Tools:**

```bash
sniff topics      # Table of subsection topics
sniff structure   # Structural overview of sniff output
```

**Hardware Details:**

```bash
sniff cpu         # CPU information
sniff gpu         # GPU information
sniff memory      # Memory information
sniff storage     # Storage/disk information
```

**Filesystem Details:**

```bash
sniff git         # Git repository information
sniff repo        # Repository/monorepo structure
sniff language    # Programming language results
sniff files       # Broad file association results
```

**Programs Subcommands:**

```bash
sniff programs                   # All installed programs
sniff editors                    # Editors (vim, VS Code, etc.)
sniff utilities                  # CLI utilities (ripgrep, fzf, etc.)
sniff language-package-managers  # Language package managers (cargo, npm, pip)
sniff os-package-managers        # OS package managers (homebrew, apt, etc.)
sniff tts-clients                # TTS clients (say, espeak, piper, etc.)
sniff terminal-apps              # Terminal apps (alacritty, wezterm, etc.)
sniff audio                      # Headless audio players (afplay, pacat, etc.)
sniff agents                     # AI agent/CLI tools (claude, kimi, etc.)
```

**Programs Output:**

```bash
# Text output (default)
sniff programs
# JSON output with full metadata
sniff programs --json
```

**Services Subcommand:**

```bash
sniff services                   # Running services (default)
sniff services --state all       # All services
sniff services --state running   # Only running services
sniff services --state stopped   # Only stopped services
sniff services --json            # JSON output
```

### Scoped Enrichment Flags

Expensive repository checks are opt-in and scoped to the subcommands that can
actually report the extra data:

```bash
# Refresh local remote-tracking data before reporting git sync status
sniff git --refresh-remotes -v

# Query registries for latest dependency versions
sniff repo --latest-versions -v

# Combine both on the aggregate filesystem report
sniff filesystem --refresh-remotes --latest-versions
```

- `sniff git --refresh-remotes` and `sniff filesystem --refresh-remotes` add:

- Remote branch lists for each git remote
- Commit synchronization status across remotes
- Detection of whether local branch is behind remote

- `sniff repo --latest-versions` and `sniff filesystem --latest-versions` add:

- Latest version information for dependencies from package registries
- Package-level update summaries in text output
- `latest_version`, `is_updatable`, and `has_major_update` fields in JSON output

## Output Examples

### Text Output (Default)

```
=== OS ===
Name: macOS 14.3.1
Kernel: Darwin 25.3.0
Architecture: aarch64
Hostname: macbook.local

Package Managers: Primary: homebrew (3 detected)

Locale: en_US.UTF-8 (UTF-8)

Timezone: America/Los_Angeles (PST, UTC-08:00)

=== Hardware ===
CPU: Apple M1 (8 logical cores)
Physical cores: 8
SIMD: NEON

Memory:
  Total: 16.0 GB
  Available: 8.5 GB
  Used: 7.5 GB

GPUs:
  Apple M1 (Apple, Metal)

Storage:
  / (apfs, SSD)

# Network
Local interface inventory for this host, plus an external WAN lookup when available.
## Snapshot
• Primary interface: en0
• WAN IP address: 203.0.113.10
• Interfaces detected: 12
• IPv4 addresses: 3
• IPv6 addresses: 6

## Interfaces
### en0
• Status: up, running, primary
• MAC address: 12:34:56:78:9a:bc
• IPv4 addresses: 192.168.1.100
• IPv6 addresses: fe80::1

=== Filesystem ===
Languages (212 contributing files out of 1,234 scanned):
  Primary: Rust
  Secondary: TypeScript, JavaScript
  Rust: 145 direct, 0 framework (68.4%)
  TypeScript: 41 direct, 26 framework (31.6%)

Files (1,234 scanned):
  Programming language: 186 files (15.1%)
  Framework file: 26 files (2.1%)
  Configuration: 54 files (4.4%)
  Documentation: 18 files (1.5%)

Git Repository:
  Root: .
  Branch: main
  HEAD: 6e11484b (Ken Snyder)
  Message: style: apply consistent formatting across workspace
  Status: dirty (3 staged, 2 unstaged, 1 untracked)
  Remote origin: GitHub (15 branches)

Monorepo: CargoWorkspace
  Packages: 8
    sniff/cli (Rust)
    sniff/lib (Rust)
    research/cli (Rust)
    ...
```

### JSON Output

```bash
# Full system info as JSON (no subcommand)
sniff | jq .
```

Returns a structured JSON object with all detection results:

```json
{
  "os": {
    "name": "macOS",
    "version": "14.3.1",
    "kernel": "Darwin 25.3.0",
    "arch": "aarch64",
    "hostname": "macbook.local",
    "system_package_managers": { ... },
    "locale": { ... },
    "time": { ... }
  },
  "hardware": {
    "cpu": { ... },
    "gpu": [ ... ],
    "memory": { ... },
    "storage": [ ... ]
  },
  "network": {
    "interfaces": [ ... ]
  },
  "filesystem": {
    "languages": { ... },
    "git": { ... },
    "repo": { ... }
  }
}
```

### Programs Output

```bash
sniff programs --json | jq .
```

Returns rich program entries (including metadata and detection status):

```json
[
  {
    "name": "Neovim",
    "binary_name": "nvim",
    "installed": true,
    "path": "/opt/homebrew/bin/nvim",
    "version": "0.10.4",
    "description": "Hyperextensible Vim-based text editor",
    "website": "https://neovim.io"
  }
]
```

### Services Output

```bash
sniff services --json | jq .
```

Returns init system and service list:

```json
{
  "init_system": "launchd",
  "services": [
    {"name": "com.apple.mDNSResponder", "running": true, "pid": 123},
    {"name": "com.docker.service", "running": true, "pid": 456}
  ]
}
```

## Architecture

### CLI Layer (`sniff/cli`)

The CLI binary provides:

- **Argument Parsing**: Uses `clap` with derive API for clean, type-safe CLI definitions
- **Subcommand Filtering**: Use subcommands like `hardware`, `cpu`, `git` to show specific sections
- **Text Rendering**: Multi-level verbosity with human-readable formatting
- **JSON Serialization**: Full structured output for programmatic use
- **Scoped Enrichment**: Async registry lookups via `--latest-versions` and remote-aware git reporting via `--refresh-remotes`

**Key Files:**

- `main.rs` - CLI argument parsing with clap subcommands, detection flow
- `output.rs` - Output rendering for text and JSON formats

### Library Layer (`sniff/lib`)

The library provides modular detection across six domains:

**OS Module:**

- Distribution and version detection
- Package manager discovery (apt, homebrew, pacman, etc.)
- Locale and timezone information
- NTP synchronization status

**Hardware Module:**

- CPU: Brand, core count, SIMD capabilities (AVX, SSE, NEON)
- GPU: Metal/Vulkan backend detection, capabilities (raytracing, mesh shaders)
- Memory: Total, available, and used bytes
- Storage: Disk type (SSD/HDD), filesystem, mount points

**Network Module:**

- Interface enumeration with permission handling
- IPv4 and IPv6 address collection
- Interface flags (up/down, loopback)

**Filesystem Module:**

1. **Git Detection** (`filesystem/git.rs`):
   - Repository root and current branch
   - Commit history with author/message
   - Dirty file tracking with diffs
   - Worktree detection and status
   - Remote provider detection (GitHub, GitLab, etc.)
   - Optional remote refresh: branch inventory, default branch, behind status, commit containment

2. **Repository Detection** (`filesystem/repo.rs`):
   - Monorepo tool detection (Cargo workspaces, pnpm, npm, yarn, Nx, Turborepo, Lerna)
   - Package enumeration with glob pattern expansion
   - Per-package language detection
   - Per-package dependency manager detection (cargo, npm, pnpm, yarn, pip, go)
   - Dependency parsing from `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, and `go.mod`

3. **Language Analysis** (`filesystem/languages.rs`):
   - File extension-based language detection
   - Percentage breakdown by file count
   - Primary language identification (excludes markup/config)

4. **Dependency Enrichment** (`package/network.rs`):
   - Async queries to package registries (crates.io, npm, PyPI)
   - Latest version resolution for `--latest-versions`
   - Manager-specific network implementations (Cargo, npm, pnpm, Yarn, Bun)

**Programs Module:**

Detects installed programs across 8 categories with parallel execution:

- **Editors**: vim, VS Code, Cursor, IntelliJ, Sublime, etc.
- **Utilities**: ripgrep, fzf, bat, jq, fd, delta, etc.
- **Language Package Managers**: cargo, npm, pip, poetry, go, etc.
- **OS Package Managers**: homebrew, apt, dnf, pacman, etc.
- **TTS Clients**: say, espeak, piper, etc.
- **Terminal Apps**: alacritty, wezterm, kitty, iTerm2, etc.
- **Headless Audio**: afplay, pacat, aplay, etc.
- **AI CLI Tools**: claude, aider, goose, etc.

Features:
- macOS app bundle detection (checks `/Applications` when PATH fails)
- `ExecutableSource` tracking (PATH vs macOS bundle)
- Version extraction via multiple strategies
- Rich metadata: display name, description, website

**Services Module:**

Detects system services across multiple init systems:

- **Supported**: systemd, launchd, OpenRC, runit, S6, Dinit, Windows SCM
- **Capabilities**: Service listing, state filtering (running/stopped), PID tracking
- **Evidence**: Tracks detection method for debugging

### Package Manager Abstraction (`package/mod.rs`)

Unified type system for package managers:

- **`OsPackageManager`**: 40+ system package managers (apt, homebrew, pacman, etc.)
- **`LanguagePackageManager`**: 70+ language ecosystem managers (cargo, npm, pip, etc.)
- **`PackageManager`**: Wrapper enum for unified handling
- **`PackageManagerShape`**: Trait for dyn-compatible package operations

**Registry System** (`package/registry.rs`):

- Global static registry of available package managers
- Runtime availability checking via PATH lookup
- Network-backed version resolution

## Technical Details

### Configuration Builder Pattern

The library uses a builder pattern for flexible configuration:

```rust
let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)
    .skip_network();

let result = detect_with_config(config)?;
```

### Verbosity Levels

Text output supports three verbosity levels:

- **Level 0** (default): Summary information, top 5 items in lists
- **Level 1** (`-v`): Extended details, full package lists, recent commits
- **Level 2** (`-vv`): Maximum detail, file lists, git diffs, EditorConfig sections

### Subcommand-Based Filtering

The CLI uses subcommands for filtering (not flags):

```bash
# Correct: use subcommands
sniff hardware
sniff cpu
sniff git

# Incorrect (old flag-based syntax, no longer supported)
# sniff --hardware
# sniff --cpu
# sniff --git
```

Each subcommand outputs text by default. Use `--json` for JSON output:

```bash
sniff cpu --json
```

### Dependency Enrichment

With `--latest-versions`, sniff enriches dependency information:

1. Parses `Cargo.toml` dependencies (normal, dev, build)
2. Extracts version requirements and features
3. Queries package registries asynchronously
4. Populates `latest_version` field in `DependencyEntry`

Supports:

- **Rust**: crates.io via API
- **JavaScript/TypeScript**: npm registry
- **Python**: PyPI JSON API
- **PHP**: Packagist search API
- **Lua**: LuaRocks HEAD requests
- **Go**: pkg.go.dev HEAD requests

### Remote Refresh

Remote-aware git inspection (`--refresh-remotes`) refreshes local remote-tracking data and reports:

- Fetches branch lists for each remote
- Determines which remotes have each recent commit
- Detects if local branch is behind remote
- Network-bound operations with error handling

### Remote Repository Inspection

The `sniff git <remote>` flow inspects remote repositories via hosting provider APIs:

```bash
# Inspect a GitHub repository
sniff git https://github.com/rust-lang/cargo

# SSH URLs also work
sniff git git@github.com:rust-lang/cargo.git

# GitLab (including nested groups)
sniff git https://gitlab.com/inkscape/inkscape

# Gitea/Codeberg
sniff git https://codeberg.org/forgejo/forgejo

# Bitbucket
sniff git https://bitbucket.org/atlassian/python-bitbucket

# JSON output
sniff git https://github.com/rust-lang/cargo --json
```

**From within a repository**, use `sniff git <name>` to inspect a configured remote by name:

```bash
# Fetch remote info for 'origin'
sniff git origin

# Or specify a URL directly
sniff git https://github.com/rust-lang/cargo
```

**Output includes:**

- Repository metadata (stars, forks, language, license)
- Open pull requests/merge requests
- Open issues
- Tags and releases
- CI/CD configuration detection
- Key URLs (issues, PRs, wiki, CI/CD, releases)

**Environment Variables:**

| Provider | Auth Env Vars |
|----------|---------------|
| GitHub | `GITHUB_TOKEN` or `GH_TOKEN` |
| GitLab | `GITLAB_TOKEN` or `GITLAB_PRIVATE_TOKEN` |
| Gitea | `GITEA_TOKEN` (or `CODEBERG_TOKEN` for Codeberg) |
| Bitbucket | `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` |

### Error Handling

- Library uses `thiserror` for structured error types
- CLI displays user-friendly error messages
- Network errors in `--refresh-remotes` and `--latest-versions` are graceful (shows available data)
- Permission denials for network interfaces are handled explicitly

## Development

### Project Structure

```
sniff/
├── cli/              # Binary crate (this package)
│   ├── src/
│   │   ├── main.rs   # CLI argument parsing, config, enrichment
│   │   └── output.rs # Text and JSON output rendering
│   └── Cargo.toml
└── lib/              # Library crate
    ├── src/
    │   ├── lib.rs                    # Public API, SniffConfig, detect()
    │   ├── os.rs                     # OS detection
    │   ├── hardware.rs               # CPU, GPU, memory, storage
    │   ├── network.rs                # Network interfaces
    │   ├── filesystem/
    │   │   ├── mod.rs                # Filesystem module
    │   │   ├── git.rs                # Git repository detection
    │   │   ├── repo.rs               # Monorepo and package detection
    │   │   └── languages.rs          # Language analysis
    │   ├── package/
    │   │   ├── mod.rs                # Package manager types
    │   │   ├── registry.rs           # Manager registry
    │   │   ├── network.rs            # Async version resolution
    │   │   └── stubs.rs              # PackageInfo type
    │   ├── programs/
    │   │   ├── mod.rs                # ProgramsInfo coordination
    │   │   ├── editors.rs            # Editor detection
    │   │   ├── utilities.rs          # CLI utility detection
    │   │   ├── pkg_mngrs.rs          # Package manager detection
    │   │   ├── tts_clients.rs        # TTS client detection
    │   │   ├── terminal_apps.rs      # Terminal emulator detection
    │   │   ├── headless_audio.rs     # Audio player detection
    │   │   ├── ai_cli.rs             # AI CLI tools detection
    │   │   ├── macos_bundle.rs       # macOS app bundle detection
    │   │   └── enums.rs              # Program enum definitions
    │   └── services/
    │       └── mod.rs                # Init system and service detection
    └── Cargo.toml
```

### Key Dependencies

- **`clap`** (4.5): Command-line argument parsing
- **`serde/serde_json`** (1.0): Serialization for JSON output and parsing
- **`tokio`** (1.48): Async runtime for network operations
- **`sysinfo`** (0.33): Cross-platform system information
- **`wgpu`** (23.0): GPU detection and capabilities
- **`git2`** (0.19): Git repository inspection
- **`toml`** (0.8): Cargo.toml parsing
- **`serde_yaml`** (0.9): pnpm-workspace.yaml parsing
- **`reqwest`** (0.12): HTTP client for registry queries

### Testing

```bash
# Run all tests
cargo test -p sniff-cli
cargo test -p sniff-lib

# Test CLI parsing
cargo test -p sniff-cli cli_parsing
```

## Use Cases

### CI/CD Integration

```bash
# Capture build environment metadata (JSON output by default)
sniff > build-context.json

# Check if running in a monorepo
if sniff repo --json | jq -e '.is_monorepo'; then
    echo "Detected monorepo"
fi
```

### Development Environment Setup

```bash
# Check available package managers
sniff os -v | grep "Package Managers"

# Verify GPU support before running ML workloads
sniff gpu --json | jq '.[0].capabilities'
```

### System Inventory

```bash
# Full system report as JSON
sniff > system-report.json

# Quick hardware summary (text output)
sniff hardware
```

### Repository Analysis

```bash
# Analyze codebase languages
sniff language -v

# Refresh remote tracking state before printing git status
sniff git --refresh-remotes -v

# Inspect dependencies with latest versions
sniff repo --latest-versions --json | jq '.packages[].dependencies'
```

## Limitations

- **Network detection** requires appropriate permissions (may fail on restricted systems)
- **`--latest-versions`** requires network access to package registries
- **`--refresh-remotes`** may contact configured git remotes and can be slow on large or private repos
- **Monorepo detection** is limited to known tools (Cargo, npm, pnpm, yarn, Nx, Turborepo, Lerna)
- **Language detection** is file extension-based (no content analysis)

## Future Enhancements

See `.ai/plans/2026-01-14.plan-for-sniff-package-roundout.md` for planned features:

- Lockfile-based resolved version extraction for npm/pnpm/yarn
- Lockfile resolution for actual versions
- Package registry abstraction layer
- Additional workspace standards beyond current built-in detectors
- Runtime environment detection (Docker, VM, cloud providers)

## License

Part of the Dockhand monorepo. See top-level LICENSE file.
