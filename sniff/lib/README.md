# Sniff Library

**sniff-lib** is a comprehensive cross-platform system and repository detection library for Rust. It provides structured, type-safe access to operating system information, hardware capabilities, network interfaces, and filesystem metadata including Git repositories and monorepo detection.

## Features

- **OS Detection**: Distribution, kernel, architecture, package managers, locale, timezone
- **Hardware Detection**: CPU with SIMD capabilities, GPU with Metal/Vulkan support, memory, storage
- **Network Detection**: Interface enumeration with IPv4/IPv6 addresses and flags
- **Filesystem Analysis**: Git repositories, monorepo tools, language detection, EditorConfig
- **Package Management**: Unified abstraction for 110+ OS and language package managers
- **Programs Detection**: 8 categories (editors, utilities, package managers, TTS, terminals, AI tools)
- **Services Detection**: Init system detection and service listing across systemd, launchd, OpenRC, etc.
- **Dependency Enrichment**: Network-based registry queries for latest versions
- **Type-Safe Errors**: Structured error types with `thiserror`
- **Serde Support**: Full serialization/deserialization for all types

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sniff-lib = { path = "../sniff/lib" }

# Optional: Enable network features for dependency enrichment
sniff-lib = { path = "../sniff/lib", features = ["network"] }
```

## Quick Start

### Basic System Detection

```rust
use sniff_lib::{detect, SniffConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Detect everything with defaults
    let result = detect()?;

    // OS information
    if let Some(os) = result.os {
        println!("OS: {} {}", os.name, os.version);
        println!("Kernel: {}", os.kernel);
        println!("Architecture: {}", os.arch);
    }

    // Hardware information
    if let Some(hw) = result.hardware {
        println!("CPU: {} ({} cores)", hw.cpu.brand, hw.cpu.logical_cores);
        println!("Memory: {} GB", hw.memory.total_bytes / (1024 * 1024 * 1024));
        println!("GPUs: {}", hw.gpu.len());
    }

    // Network information
    if let Some(net) = result.network {
        println!("Primary interface: {:?}", net.primary_interface);
        println!("Interfaces: {}", net.interfaces.len());
    }

    // Filesystem information
    if let Some(fs) = result.filesystem {
        if let Some(git) = fs.git {
            println!("Git repo: {:?}", git.repo_root);
            println!("Branch: {:?}", git.current_branch);
        }
    }

    Ok(())
}
```

### Configuration Builder

```rust
use sniff_lib::SniffConfig;
use std::path::PathBuf;

let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)              // Enable deep git inspection
    .skip_network();         // Skip network detection

let result = sniff_lib::detect_with_config(config)?;
```

### Selective Detection

```rust
use sniff_lib::{
    hardware::detect_hardware,
    network::detect_network,
    os::detect_os,
};

// Detect only hardware
let hw = detect_hardware()?;
println!("CPU: {}", hw.cpu.brand);

// Detect only network
let net = detect_network()?;
for iface in &net.interfaces {
    println!("Interface: {}", iface.name);
}

// Detect only OS
let os = detect_os()?;
println!("OS: {} {}", os.name, os.version);
```

## Architecture

### Module Organization

```
sniff-lib/
├── os              # Operating system detection
├── hardware        # CPU, GPU, memory, storage
├── network         # Network interfaces
├── filesystem      # Git, monorepo, languages
├── package         # Package manager abstraction
├── programs        # Installed program detection (8 categories)
├── services        # System service and init system detection
└── error           # Error types
```

### Core Types

#### `SniffResult`

The top-level result type containing all detection data:

```rust
pub struct SniffResult {
    pub os: Option<OsInfo>,
    pub hardware: Option<HardwareInfo>,
    pub network: Option<NetworkInfo>,
    pub filesystem: Option<FilesystemInfo>,
}
```

#### `SniffConfig`

Builder for configuring detection behavior:

```rust
pub struct SniffConfig {
    pub base_dir: Option<PathBuf>,
    pub include_cpu_usage: bool,
    pub deep: bool,               // Enable deep git inspection
    pub skip_os: bool,
    pub skip_hardware: bool,
    pub skip_network: bool,
    pub skip_filesystem: bool,
}
```

## Module Reference

### OS Module

Detects operating system information across Windows, macOS, Linux, and BSD systems.

**Key Types:**

- `OsInfo` - Complete OS information
- `OsType` - OS classification (Windows, Linux, macOS, etc.)
- `LinuxDistro` - Linux distribution details with family classification
- `LinuxFamily` - Distribution family (Debian, RedHat, Arch, etc.)
- `SystemPackageManagers` - Detected system package managers
- `LocaleInfo` - Locale and encoding information
- `TimeInfo` - Timezone, UTC offset, NTP status, DST

**Detection Strategy:**

1. **OS Type**: Runtime detection via `std::env::consts::OS`
2. **Linux Distribution**: Parses `/etc/os-release`, `/etc/lsb-release`, `/etc/system-release`
3. **Package Managers**: PATH-based detection of 40+ package managers
4. **Locale**: Parses `LC_ALL`, `LANG` environment variables
5. **Timezone**: System API queries for timezone, offset, NTP status

**Example:**

```rust
use sniff_lib::os::{detect_os, detect_linux_distro};

let os = detect_os()?;
println!("OS: {} {}", os.name, os.version);
println!("Architecture: {}", os.arch);

if let Some(pkg_mgrs) = os.system_package_managers {
    println!("Primary package manager: {:?}", pkg_mgrs.primary);
    for pm in &pkg_mgrs.managers {
        println!("  - {} at {}", pm.manager, pm.path);
    }
}

if let Some(locale) = os.locale {
    println!("Locale: {:?}", locale.lang);
    println!("Encoding: {:?}", locale.encoding);
}
```

### Hardware Module

Cross-platform hardware detection with detailed CPU, GPU, memory, and storage information.

**Key Types:**

- `HardwareInfo` - Aggregate hardware information
- `CpuInfo` - CPU brand, cores, SIMD capabilities
- `SimdCapabilities` - SIMD instruction sets (SSE, AVX, AVX-512, NEON)
- `GpuInfo` - GPU details with capabilities
- `GpuCapabilities` - Raytracing, mesh shaders, unified memory
- `MemoryInfo` - Total, available, used memory
- `StorageInfo` - Disk type (SSD/HDD), filesystem, mount point

**SIMD Detection:**

Uses architecture-specific intrinsics:

- **x86_64**: SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2, AVX, AVX2, AVX-512, FMA
- **aarch64**: NEON

**GPU Detection:**

- **macOS**: Metal API with full capability detection
- **Other platforms**: Returns empty vector (future: Vulkan/D3D12 support)

**Example:**

```rust
use sniff_lib::hardware::{detect_hardware, SimdCapabilities};

let hw = detect_hardware()?;

// CPU with SIMD capabilities
let cpu = &hw.cpu;
println!("CPU: {} ({} logical, {:?} physical)",
    cpu.brand,
    cpu.logical_cores,
    cpu.physical_cores
);

if cpu.simd.avx2 {
    println!("AVX2 supported - can use 256-bit vectors");
}

// GPU capabilities
for gpu in &hw.gpu {
    println!("GPU: {} ({:?})", gpu.name, gpu.device_type);
    println!("  Backend: {}", gpu.backend);
    if let Some(mem) = gpu.memory_bytes {
        println!("  Memory: {} GB", mem / (1024 * 1024 * 1024));
    }
    println!("  Raytracing: {}", gpu.capabilities.raytracing);
    println!("  Unified Memory: {}", gpu.capabilities.unified_memory);
}

// Storage
for disk in &hw.storage {
    println!("Disk: {} ({:?})", disk.mount_point.display(), disk.kind);
    println!("  Total: {} GB", disk.total_bytes / (1024 * 1024 * 1024));
}
```

### Network Module

Network interface enumeration using `getifaddrs` system call.

**Key Types:**

- `NetworkInfo` - All interfaces with primary detection
- `NetworkInterface` - Interface name, addresses, flags
- `InterfaceFlags` - Up/down, loopback, running status

**Features:**

- IPv4 and IPv6 address collection
- Primary interface detection (first non-loopback with IPv4)
- Permission denied error handling
- Interface filtering utilities

**Example:**

```rust
use sniff_lib::network::{detect_network, detect_network_filtered};

// All interfaces
let net = detect_network()?;
if !net.permission_denied {
    for iface in &net.interfaces {
        println!("Interface: {}", iface.name);
        println!("  Up: {}, Loopback: {}", iface.flags.is_up, iface.flags.is_loopback);
        for addr in &iface.ipv4_addresses {
            println!("  IPv4: {}", addr);
        }
    }
}

// Only active, non-loopback interfaces
let filtered = detect_network_filtered()?;
// All interfaces here are up and not loopback
```

### Filesystem Module

Comprehensive filesystem analysis including Git, monorepo detection, and language breakdown.

**Submodules:**

1. **Git Detection** (`filesystem::git`)
2. **Repository Detection** (`filesystem::repo`)
3. **Language Analysis** (`filesystem::languages`)
4. **EditorConfig** (`filesystem::formatting`)

#### Git Detection

Uses `libgit2` (via `git2` crate) for repository inspection.

**Key Types:**

- `GitInfo` - Complete git repository information
- `CommitInfo` - Commit SHA, author, message, remote sync status
- `RepoStatus` - Dirty files, staged/unstaged/untracked counts
- `RemoteInfo` - Remote name, URL, provider, branches (deep mode)
- `HostingProvider` - GitHub, GitLab, Bitbucket, etc.
- `BehindStatus` - Whether local branch is behind remote
- `WorktreeInfo` - Linked worktree information

**Detection Strategy:**

- **Standard Mode**: Local repository inspection (no network)
- **Deep Mode** (`--deep`): Queries remotes for branch lists and commit synchronization

**Example:**

```rust
use sniff_lib::filesystem::git::detect_git;
use std::path::Path;

// Standard mode (no network)
let git = detect_git(Path::new("."), false)?;
if let Some(info) = git {
    println!("Repository: {:?}", info.repo_root);
    println!("Branch: {:?}", info.current_branch);
    println!("Dirty: {}", info.status.is_dirty);
    println!("Commits ahead: {}", info.recent.len());

    for commit in info.recent.iter().take(5) {
        println!("  {} - {}", &commit.sha[..8], commit.message.lines().next().unwrap_or(""));
    }
}

// Deep mode (queries remotes)
let git_deep = detect_git(Path::new("."), true)?;
if let Some(info) = git_deep {
    for remote in &info.remotes {
        println!("Remote {}: {:?}", remote.name, remote.provider);
        if let Some(ref branches) = remote.branches {
            println!("  Branches: {}", branches.len());
        }
    }

    // Check if behind
    if let Some(ref behind) = info.status.is_behind {
        match behind {
            sniff_lib::filesystem::git::BehindStatus::NotBehind => {
                println!("Up to date with remotes");
            }
            sniff_lib::filesystem::git::BehindStatus::Behind(remotes) => {
                println!("Behind: {}", remotes.join(", "));
            }
        }
    }
}
```

#### Repository Detection

Detects monorepo tools and package structure.

**Supported Tools:**

- Cargo workspaces (Rust)
- pnpm workspaces
- npm workspaces
- Yarn workspaces
- Nx
- Turborepo
- Lerna

**Key Types:**

- `RepoInfo` - Repository metadata and packages
- `MonorepoTool` - Detected monorepo tool
- `PackageLocation` - Package path, languages, managers, dependencies
- `DependencyEntry` - Dependency with version requirements

**Example:**

```rust
use sniff_lib::filesystem::repo::detect_repo;
use std::path::Path;

let repo = detect_repo(Path::new("."))?;
if let Some(info) = repo {
    if info.is_monorepo {
        println!("Monorepo tool: {:?}", info.monorepo_tool);
        if let Some(packages) = info.packages {
            println!("Packages: {}", packages.len());
            for pkg in packages {
                println!("  {} at {}", pkg.name, pkg.path.display());
                println!("    Language: {:?}", pkg.primary_language);
                println!("    Managers: {:?}", pkg.detected_managers);

                if let Some(deps) = pkg.dependencies {
                    println!("    Dependencies:");
                    for dep in deps.iter().take(5) {
                        println!("      - {} @ {}", dep.name, dep.targeted_version);
                    }
                }
            }
        }
    }
}
```

#### Language Analysis

File extension-based language detection.

**Key Types:**

- `LanguageBreakdown` - Complete language statistics
- `LanguageStats` - Per-language file count and percentage

**Example:**

```rust
use sniff_lib::filesystem::languages::detect_languages;
use std::path::Path;

let langs = detect_languages(Path::new("."))?;
println!("Files analyzed: {}", langs.total_files);
println!("Primary language: {:?}", langs.primary);

for lang in langs.languages.iter().take(5) {
    println!("{}: {} files ({:.1}%)",
        lang.language,
        lang.file_count,
        lang.percentage
    );
}
```

### Package Module

Unified abstraction for operating system and language package managers.

**Key Types:**

- `OsPackageManager` - 40+ system package managers (apt, homebrew, pacman, etc.)
- `LanguagePackageManager` - 70+ language managers (cargo, npm, pip, etc.)
- `PackageManager` - Wrapper enum for both types
- `PackageManagerShape` - Trait for dyn-compatible package operations

**Network Support:**

When the `network` feature is enabled, provides async registry queries:

- `CargoNetwork` - crates.io API queries
- `NpmNetwork` - npm registry API queries
- `PnpmNetwork`, `YarnNetwork`, `BunNetwork` - Use npm registry
- `enrich_dependencies()` - Fetch latest versions for dependency lists

**Example:**

```rust
use sniff_lib::package::{
    OsPackageManager, LanguagePackageManager, PackageManager,
    get_package_manager, is_registered,
};

// Unified type
let managers = vec![
    PackageManager::Os(OsPackageManager::Apt),
    PackageManager::Language(LanguagePackageManager::Npm),
];

for mgr in &managers {
    println!("{} (executable: {})", mgr, mgr.executable_name());
}

// Check if registered
if is_registered(LanguagePackageManager::Cargo.executable_name()) {
    let mgr = get_package_manager("cargo").unwrap();
    if mgr.is_available() {
        println!("Cargo is available");
    }
}
```

**Network Enrichment** (requires `network` feature):

```rust
use sniff_lib::package::{enrich_dependencies, DependencyEntry, DependencyKind};

let mut deps = vec![
    DependencyEntry {
        name: "serde".to_string(),
        kind: DependencyKind::Normal,
        targeted_version: "1.0".to_string(),
        package_manager: Some("cargo".to_string()),
        latest_version: None,
        // ... other fields
    },
];

// Fetch latest versions from registries
let enriched = enrich_dependencies(deps).await;
for dep in &enriched {
    if let Some(ref latest) = dep.latest_version {
        println!("{}: {} (latest: {})", dep.name, dep.targeted_version, latest);
    }
}
```

### Programs Module

Detects installed programs across 8 categories with parallel execution and macOS app bundle support.

**Key Types:**

- `ProgramsInfo` - Aggregated detection results for all categories
- `ProgramMetadata` - Trait for program metadata (display name, description, website)
- `ExecutableSource` - How program was discovered (PATH vs macOS bundle)
- `InstallOptions`, `InstallResult` - Installation infrastructure types

**Categories:**

| Category | Examples | Detection |
|----------|----------|-----------|
| Editors | vim, VS Code, Cursor, IntelliJ | PATH + macOS bundles |
| Utilities | ripgrep, fzf, bat, jq, fd | PATH lookup |
| Language PMs | cargo, npm, pip, poetry | PATH lookup |
| OS PMs | homebrew, apt, dnf, pacman | PATH lookup |
| TTS Clients | say, espeak, piper | PATH + macOS bundles |
| Terminal Apps | alacritty, wezterm, kitty | PATH + macOS bundles |
| Headless Audio | afplay, pacat, aplay | PATH lookup |
| AI CLI | claude, aider, goose | PATH lookup |

**Example:**

```rust
use sniff_lib::programs::ProgramsInfo;

// Detect all installed programs (parallel)
let programs = ProgramsInfo::detect();

println!("Editors: {:?}", programs.editors);
println!("Utilities: {:?}", programs.utilities);
println!("AI CLI tools: {:?}", programs.ai_cli);

// Access metadata
for editor in &programs.editors {
    println!("{}: {}", editor.display_name(), editor.description());
}
```

**macOS App Bundle Detection:**

```rust
use sniff_lib::programs::find_program_with_source;

// Returns (Option<PathBuf>, ExecutableSource)
let (path, source) = find_program_with_source("code");
match source {
    ExecutableSource::Path => println!("Found in PATH"),
    ExecutableSource::MacOsBundle(bundle) => println!("Found in {}", bundle),
    ExecutableSource::NotFound => println!("Not installed"),
}
```

### Services Module

Detects system services across multiple init systems.

**Key Types:**

- `Services` - Init system detection result with service list
- `ServiceInfo` - Individual service (name, running state, PID)
- `ServiceState` - Filter enum (All, Running, Stopped)
- `InitSystem` - Detected init system (systemd, launchd, OpenRC, etc.)

**Supported Init Systems:**

- systemd (Linux)
- launchd (macOS)
- OpenRC (Gentoo, Alpine)
- runit (Void Linux)
- S6 (s6-rc)
- Dinit
- Windows SCM

**Example:**

```rust
use sniff_lib::services::{detect_services, ServiceState};

let services = detect_services();

if let Some(init) = &services.init_system {
    println!("Init system: {:?}", init);
}

// Filter by state
let running: Vec<_> = services.services
    .iter()
    .filter(|s| s.running)
    .collect();

println!("Running services: {}", running.len());
```

## Error Handling

The library uses `thiserror` for structured error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SniffError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    #[error("System info error in {domain}: {message}")]
    SystemInfo {
        domain: &'static str,
        message: String,
    },

    #[error("Language detection failed: {0}")]
    LanguageDetection(String),
}

pub type Result<T> = std::result::Result<T, SniffError>;
```

**Error Handling Strategy:**

- Network errors in `--deep` mode return partial results (graceful degradation)
- Permission denied for network interfaces returns `NetworkInfo` with flag set
- Git operations fail fast with structured errors
- All public functions return `Result<T>`

## Feature Flags

```toml
[features]
default = []
network = ["reqwest"]  # Enable network-based registry queries
```

**Network Feature:**

When enabled, provides:

- `CargoNetwork`, `NpmNetwork`, etc. implementations
- `enrich_dependencies()` async function
- Latest version resolution from package registries

## Platform Support

| Platform | OS Detection | Hardware | Network | Git | GPU |
|----------|:------------:|:--------:|:-------:|:---:|:---:|
| **Linux** | ✓ | ✓ | ✓ | ✓ | - |
| **macOS** | ✓ | ✓ | ✓ | ✓ | ✓ (Metal) |
| **Windows** | ✓ | ✓ | ✓ | ✓ | - |
| **BSD** | ✓ | ✓ | ✓ | ✓ | - |

**Notes:**

- GPU detection currently only supported on macOS (Metal API)
- Network interface detection requires appropriate permissions
- Git operations require valid repository

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `sysinfo` | 0.33 | CPU, memory, storage detection |
| `git2` | 0.19 | Git repository inspection |
| `wgpu` | 23.0 | GPU detection (future cross-platform) |
| `metal` | 0.30 | macOS GPU detection |
| `getifaddrs` | 0.2 | Network interface enumeration |
| `toml` | 0.8 | Cargo.toml parsing |
| `serde_yaml` | 0.9 | pnpm-workspace.yaml parsing |
| `reqwest` | 0.12 | HTTP client (network feature) |
| `thiserror` | 2.0 | Error type derivation |
| `serde` | 1.0 | Serialization support |

## Testing

The library includes comprehensive unit tests for all modules:

```bash
# Run all tests
cargo test -p sniff-lib

# Test specific modules
cargo test -p sniff-lib os::
cargo test -p sniff-lib hardware::
cargo test -p sniff-lib network::
cargo test -p sniff-lib filesystem::

# Test with network feature
cargo test -p sniff-lib --features network
```

**Test Coverage:**

- OS detection: Distribution parsing, package manager detection
- Hardware: SIMD capabilities, serialization roundtrips
- Network: Interface detection, filtering, primary selection
- Filesystem: Git parsing, monorepo detection, language analysis
- Package: Manager detection, registry queries (network)

## Design Principles

1. **Zero-Cost Abstractions**: Detection is opt-in via configuration
2. **Graceful Degradation**: Network errors don't fail the entire detection
3. **Type Safety**: Strong typing with `thiserror` errors and `serde` serialization
4. **Cross-Platform**: Uniform API across Windows, macOS, Linux, BSD
5. **No Unwrap**: All production code uses proper error handling
6. **Async-Ready**: Network operations use `tokio` for concurrent queries

## Use Cases

### CI/CD Environment Detection

```rust
let result = detect()?;
let json = serde_json::to_string_pretty(&result)?;
std::fs::write("build-context.json", json)?;
```

### Dependency Auditing

```rust
use sniff_lib::{filesystem::repo::detect_repo, package::enrich_dependencies};

let repo = detect_repo(Path::new("."))?;
if let Some(info) = repo {
    if let Some(packages) = info.packages {
        for pkg in packages {
            if let Some(deps) = pkg.dependencies {
                let enriched = enrich_dependencies(deps).await;
                for dep in enriched {
                    if let Some(latest) = dep.latest_version {
                        if latest != dep.targeted_version {
                            println!("{}: {} -> {}", dep.name, dep.targeted_version, latest);
                        }
                    }
                }
            }
        }
    }
}
```

### Hardware Capability Checks

```rust
let hw = detect_hardware()?;
if hw.cpu.simd.avx2 && hw.memory.total_bytes >= 16 * 1024 * 1024 * 1024 {
    println!("System meets requirements for ML workloads");
}
```

## Future Enhancements

See `.ai/plans/2026-01-14.plan-for-sniff-package-roundout.md` for planned features:

- Expanded dependency parsing (npm, pip, go.mod)
- Lockfile resolution for actual versions
- GPU detection for Windows (D3D12) and Linux (Vulkan)
- Runtime environment detection (Docker, VM, cloud providers)
- Performance profiling and benchmarking

## License

Part of the Dockhand monorepo. See top-level LICENSE file.
