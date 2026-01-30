# Sniff CLI

**Sniff** is a comprehensive system and repository detection tool that provides detailed information about your operating system, hardware, network, and filesystem environment. It's designed to give developers, system administrators, and automation tools a complete snapshot of the execution context.

## Features

- **OS Detection**: Distribution, kernel, architecture, hostname, package managers, locale, timezone, and NTP status
- **Hardware Detection**: CPU (with SIMD capabilities), GPU (with Metal/Vulkan support), memory, and storage
- **Network Detection**: Network interfaces with IPv4/IPv6 addresses and status flags
- **Filesystem Detection**: Git repository status, monorepo detection, programming language analysis, and EditorConfig formatting rules
- **Dependency Enrichment**: Fetch latest versions from package registries with `--deep` mode
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
# Detect everything (default)
sniff

# Detect with a specific base directory
sniff --base /path/to/project

# Get JSON output
sniff --json

# Enable verbose output (show more details)
sniff -v        # Level 1: more details
sniff -vv       # Level 2: even more details
```

### Section Selection

**Include-Only Mode** (combine multiple sections):

```bash
# Show only hardware section
sniff --hardware
# Show only filesystem section
sniff --filesystem
# Combine network and filesystem
sniff --network --filesystem
```

**Skip Mode** (when no include flags specified):

```bash
# Skip hardware detection
sniff --skip-hardware
# Skip network detection
sniff --skip-network
# Skip filesystem detection
sniff --skip-filesystem
```

### Filter Flags (Mutually Exclusive)

Filter flags show only specific subsections. Only one filter flag can be used at a time.

**Top-Level Filters:**

```bash
# Show only OS information
sniff --os
```

**Hardware Subsection Filters:**

```bash
# Show only CPU information
sniff --cpu
# Show only GPU information
sniff --gpu
# Show only memory information
sniff --memory
# Show only storage information
sniff --storage
```

**Filesystem Subsection Filters:**

```bash
# Show only git repository information
sniff --git
# Show only repository/monorepo structure
sniff --repo
# Show only language detection results
sniff --language
```

**Programs Filters:**

```bash
# Show all installed programs
sniff --programs
# Show only editors (vim, vs code, etc.)
sniff --editors
# Show only CLI utilities (ripgrep, fzf, etc.)
sniff --utilities
# Show only language package managers (cargo, npm, pip, etc.)
sniff --language-package-managers
# Show only OS package managers (homebrew, apt, etc.)
sniff --os-package-managers
# Show only TTS clients (say, espeak, piper, etc.)
sniff --tts-clients
# Show only terminal apps (alacritty, wezterm, etc.)
sniff --terminal-apps
# Show only headless audio players (afplay, pacat, etc.)
sniff --audio
```

**Programs Output Formats:**

```bash
# Markdown table output (default for programs)
sniff --programs --markdown
# JSON output with simple format (backward compatible)
sniff --programs --json
# JSON output with full metadata
sniff --programs --json --json-format full
```

**Services Filter:**

```bash
# Show system services
sniff --services
# Filter by service state
sniff --services --state all       # All services (default)
sniff --services --state running   # Only running services
sniff --services --state stopped   # Only stopped services
```

**Note:** Filter flags are mutually exclusive. For example, `sniff --cpu --memory` will error because you can only specify one filter at a time.

### Deep Mode

Enable deep inspection for enhanced repository information:

```bash
# Enable deep git inspection (queries remote branches)
sniff --deep

# Show git info with remote branch details
sniff --git --deep -v
```

Deep mode provides:

- Remote branch lists for each git remote
- Commit synchronization status across remotes
- Detection of whether local branch is behind remote
- Latest version information for dependencies from package registries

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

=== Network ===
Primary interface: en0
Interfaces: 12
  en0 [UP]
    IPv4: 192.168.1.100
    IPv6: fe80::1

=== Filesystem ===
Languages (1,234 files analyzed):
  Primary: Rust
  Rust: 145 files (62.3%)
  TypeScript: 67 files (28.8%)
  JavaScript: 21 files (9.0%)

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
sniff --json | jq .
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
sniff --programs --json | jq .
```

Returns installed programs organized by category:

```json
{
  "editors": ["vim", "code", "cursor"],
  "utilities": ["ripgrep", "fzf", "bat", "jq"],
  "language_package_managers": ["cargo", "npm", "pip"],
  "os_package_managers": ["homebrew"],
  "tts_clients": ["say"],
  "terminal_apps": ["wezterm", "alacritty"],
  "headless_audio": ["afplay"],
  "ai_cli": ["claude", "aider"]
}
```

With `--json-format full`, includes rich metadata (display name, description, website, version, source).

### Services Output

```bash
sniff --services --json | jq .
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
- **Output Filtering**: Two modes for controlling output:
    - **Include-Only Mode**: Combine `--hardware`, `--network`, `--filesystem` flags
    - **Filter Mode**: Mutually exclusive detail filters like `--cpu`, `--git`, `--repo`
- **Text Rendering**: Multi-level verbosity with human-readable formatting
- **JSON Serialization**: Full structured output for programmatic use
- **Dependency Enrichment**: Async network queries to package registries in `--deep` mode

**Key Files:**

- `main.rs:219-310` - Main detection flow and configuration
- `main.rs:312-359` - Dependency enrichment for `--deep` mode
- `output.rs:79-169` - Output rendering with filter support
- `main.rs:89-180` - Filter flag validation and selection logic

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
   - Deep mode: Remote branch lists, commit synchronization

2. **Repository Detection** (`filesystem/repo.rs`):
   - Monorepo tool detection (Cargo workspaces, pnpm, npm, Nx, Turborepo, Lerna)
   - Package enumeration with glob pattern expansion
   - Per-package language detection
   - Per-package dependency manager detection (cargo, npm, pnpm, yarn, pip, go)
   - Dependency parsing from `Cargo.toml` with version requirements

3. **Language Analysis** (`filesystem/languages.rs`):
   - File extension-based language detection
   - Percentage breakdown by file count
   - Primary language identification (excludes markup/config)

4. **Dependency Enrichment** (`package/network.rs`):
   - Async queries to package registries (crates.io, npm, PyPI)
   - Latest version resolution for `--deep` mode
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

### Filter Mode vs Include-Only Mode

**Include-Only Mode** (combinable):

- Triggered by `--hardware`, `--network`, `--filesystem`
- Multiple flags can be combined
- Skip flags are ignored in this mode

**Filter Mode** (mutually exclusive):

- Triggered by detail-level flags like `--cpu`, `--git`, `--repo`
- Only one filter flag allowed at a time
- Shows only the specific subsection requested

### Dependency Enrichment

With `--deep` mode, sniff enriches dependency information:

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

### Git Deep Mode

Deep git inspection (`--deep`) queries remote repositories:

- Fetches branch lists for each remote
- Determines which remotes have each commit (via `git branch -r --contains`)
- Detects if local branch is behind remote
- Network-bound operations with error handling

### Error Handling

- Library uses `thiserror` for structured error types
- CLI displays user-friendly error messages
- Network errors in `--deep` mode are graceful (shows available data)
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

# Test filter flag validation
cargo test -p sniff-cli filter_flag_validation

# Test CLI parsing
cargo test -p sniff-cli cli_parsing
```

## Use Cases

### CI/CD Integration

```bash
# Capture build environment metadata
sniff --json > build-context.json

# Check if running in a monorepo
if sniff --repo --json | jq -e '.filesystem.repo.is_monorepo'; then
    echo "Detected monorepo"
fi
```

### Development Environment Setup

```bash
# Check available package managers
sniff --os -v | grep "Package Managers"

# Verify GPU support before running ML workloads
sniff --gpu --json | jq '.hardware.gpu[0].capabilities'
```

### System Inventory

```bash
# Full system report with maximum verbosity
sniff -vv > system-report.txt

# Quick hardware summary
sniff --hardware
```

### Repository Analysis

```bash
# Analyze codebase languages
sniff --language -v

# Check git status across monorepo packages
sniff --git --deep -v

# Inspect dependencies with latest versions
sniff --repo --deep --json | jq '.filesystem.repo.packages[].dependencies'
```

## Limitations

- **Network detection** requires appropriate permissions (may fail on restricted systems)
- **Deep mode** requires network access to package registries
- **Git deep mode** queries all remotes (can be slow for many remotes)
- **Monorepo detection** is limited to known tools (Cargo, npm, pnpm, Nx, Turborepo, Lerna)
- **Language detection** is file extension-based (no content analysis)

## Future Enhancements

See `.ai/plans/2026-01-14.plan-for-sniff-package-roundout.md` for planned features:

- Expanded dependency parsing (npm, pnpm, pip, go.mod)
- Lockfile resolution for actual versions
- Package registry abstraction layer
- Extended monorepo tool support
- Runtime environment detection (Docker, VM, cloud providers)

## License

Part of the Dockhand monorepo. See top-level LICENSE file.
