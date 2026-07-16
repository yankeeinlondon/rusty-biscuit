# Sniff Library

**sniff** is a comprehensive cross-platform system and repository detection library for Rust. It provides structured, type-safe access to operating system information, hardware capabilities, network interfaces, and filesystem metadata including Git repositories and monorepo detection.

## Features

- **OS Detection**: Distribution, kernel, architecture, package managers, locale, timezone
- **Hardware Detection**: CPU with SIMD capabilities, GPU with Metal/Vulkan support, memory, storage, audio devices
- **Network Detection**: Interface enumeration with IPv4/IPv6 addresses, flags, and WAN IP lookup
- **Filesystem Analysis**: Git repositories, monorepo tools, structured language detection, broad file associations, EditorConfig, blast radius, justfile detection, recent commits
- **Package Management**: Unified abstraction for 110+ OS and language package managers
- **Programs Detection**: 9 categories (editors, utilities, package managers, TTS, terminals, AI tools, test runners)
- **Services Detection**: Init system detection and service listing across systemd, launchd, OpenRC, etc.
- **Dependency Enrichment**: Network-based registry queries for latest versions
- **Type-Safe Errors**: Structured error types with `thiserror`
- **Serde Support**: Full serialization/deserialization for all types

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sniff = { path = "../sniff/lib" }

# Optional: Enable network features for dependency enrichment
sniff = { path = "../sniff/lib", features = ["network"] }
```

## Quick Start

### Simple Detection

```rust
use sniff::detect;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = detect()?;

    if let Some(os) = result.os {
        println!("OS: {} {}", os.name, os.version);
    }
    if let Some(hw) = result.hardware {
        println!("CPU: {} ({})", hw.cpu.brand, hw.cpu.arch);
        println!("Memory: {} GB", hw.memory.total_bytes / (1024 * 1024 * 1024));
    }

    Ok(())
}
```

### Plan-Based Detection (DetectionPlan)

`DetectionPlan` is the primary API for callers who need cost control. Each domain
(OS, hardware, network, filesystem) can be included at a chosen detail level or
excluded entirely, and filesystem sub-sections (git, repo, docs, etc.) compose
independently.

```rust
use sniff::{detect_with_plan, request::*};

let plan = DetectionPlan::new()
    .os(OsRequest::summary())
    .hardware(HardwareRequest::summary())
    .without_network()
    .filesystem(FilesystemRequest::new()
        .git(GitRequest::summary())
        .repo(RepoRequest::structure())
        .without_docs());

let result = detect_with_plan(plan)?;
```

### Legacy SniffConfig

`SniffConfig` predates `DetectionPlan` and offers coarser boolean toggles. It
remains supported but cannot express per-subsection detail levels.

```rust
use sniff::{detect_with_config, SniffConfig};
use std::path::PathBuf;

let config = SniffConfig::new()
    .base_dir(PathBuf::from("."))
    .deep(true)
    .commit_count(20)
    .skip_network();

let result = detect_with_config(config)?;
```

### Module-Level Detection

```rust
use sniff::{
    hardware::{detect_hardware, detect_hardware_summary},
    network::detect_network,
    os::{detect_os, detect_os_with_request},
    filesystem::git::GitRepo,
    filesystem::repo::detect_repo_structure,
    request::OsRequest,
};
use std::path::Path;

// Detect only hardware (full, includes audio + GPU + storage)
let hw = detect_hardware()?;
println!("CPU: {} ({})", hw.cpu.brand, hw.cpu.arch);

// Lightweight hardware summary: CPU + memory only, skips ~1.5s audio detection
let hw_summary = detect_hardware_summary()?;

// Detect only network
let net = detect_network()?;
for iface in &net.interfaces {
    println!("Interface: {}", iface.name);
}

// OS with tuned detail level (skip NTP which can take up to 10s on Linux)
let os = detect_os_with_request(&OsRequest::summary())?;
println!("OS: {} {}", os.name, os.version);

// Expert composition: discover a git repo and sniff workspace structure only
let git = GitRepo::discover(Path::new("."))?;
let repo = detect_repo_structure(Path::new("."))?;
```

See [../docs/sniff-library-architecture.md](../docs/sniff-library-architecture.md) for a full breakdown of per-subsection costs, shared-work strategies, and common caller profiles.

## Architecture

### Module Organization

```
sniff/
├── os/             # Operating system detection (distro, locale, time, package managers)
├── hardware/       # CPU, GPU, memory, storage, audio devices
├── network/        # Network interfaces
├── filesystem/     # Git, monorepo, languages, file types, docs, blast radius, just
├── package/        # Package manager abstraction (110+)
├── programs/       # Installed program detection and install (9 categories)
├── remote/         # Remote repo inspection (GitHub, GitLab, Gitea, Bitbucket)
├── services/       # System service and init system detection
├── request         # Fine-grained detection control (DetectionPlan, request types)
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

#### `DetectionPlan`

Fine-grained request controlling which domains are collected and at what detail level. Each domain accepts a typed request (`OsRequest`, `HardwareRequest`, etc.) or can be excluded with `without_*()`. See `sniff::request` for all request types.

```rust
pub struct DetectionPlan {
    pub base_dir: Option<PathBuf>,
    pub os: Option<OsRequest>,
    pub hardware: Option<HardwareRequest>,
    pub network: Option<NetworkRequest>,
    pub filesystem: Option<FilesystemRequest>,
}
```

#### `SniffConfig`

Legacy builder with coarser boolean toggles:

```rust
pub struct SniffConfig {
    pub base_dir: Option<PathBuf>,
    pub deep: bool,               // Enable deep git inspection
    pub commit_count: usize,      // Recent commits to retrieve (default: 10)
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
- `NtpStatus` - NTP synchronization state (Synchronized, NotSynchronized, Unknown)

**Detection Strategy:**

1. **OS Type**: Runtime detection via `std::env::consts::OS`
2. **Linux Distribution**: Parses `/etc/os-release`, `/etc/lsb-release`, `/etc/system-release`
3. **Package Managers**: PATH-based detection of 40+ package managers
4. **Locale**: Parses `LC_ALL`, `LANG` environment variables
5. **Timezone**: System API queries for timezone, offset, DST
6. **NTP Status**: Platform-specific detection (macOS: `sntp`, Windows: `w32tm`, Linux: `timedatectl`)

**Timezone API cost:** `detect_timezone()` reports the **full** `TimeInfo`,
which means it runs the NTP probe in step 6 — a network round-trip that can take
several seconds (up to ~10 s on Linux). Callers that need only local timezone
data should call `detect_timezone_with_options(false)`, which returns
immediately with `ntp_status: NtpStatus::Unknown` and spawns no external
command. Steps 1–5 are identical either way.

**Example:**

```rust
use sniff::os::{detect_os, detect_linux_distro};

let os = detect_os()?;
println!("OS: {} {}", os.name, os.version);
println!("Kernel: {}", os.kernel);

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
- `AudioDevices` - Input and output audio devices with sample rates
- `AudioDeviceInfo` - Individual audio device details

**SIMD Detection:**

Uses architecture-specific intrinsics:

- **x86_64**: SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2, AVX, AVX2, AVX-512, FMA
- **aarch64**: NEON

**GPU Detection:**

- **macOS**: Metal API with full capability detection
- **Other platforms**: Returns empty vector (future: Vulkan/D3D12 support)

**Audio Device Detection:**

- **macOS**: Core Audio API for input/output devices with sample rates
- **Linux**: PulseAudio (`pactl`) and ALSA (`arecord`/`aplay`) enumeration
- **Windows**: PowerShell `Get-AudioDevice` or `Get-PnpDevice` fallback

**Example:**

```rust
use sniff::hardware::{detect_hardware, SimdCapabilities};

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
- Primary interface detection (default-route interface when detectable)
- WAN IP lookup when the `network` feature is enabled
- Permission denied error handling
- Interface filtering utilities

**Example:**

```rust
use sniff::network::{detect_network, detect_network_filtered};

// All interfaces
let net = detect_network()?;
if !net.permission_denied {
    println!("WAN IP: {:?}", net.wan_ip_address);
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

Comprehensive filesystem analysis including Git, monorepo detection, language breakdown, and markdown document discovery.

**Submodules:**

1. **Git Detection** (`filesystem::git`) - Repository status, commits, worktrees, remotes
2. **Repository Detection** (`filesystem::repo`) - Monorepo tools, package enumeration
3. **Language Analysis** (`filesystem::languages`) - File extension-based language detection
4. **File Type Classification** (`filesystem::file_types`) - Broad file association (programming, config, docs, etc.)
5. **EditorConfig** (`filesystem::formatting`) - Formatting rule detection
6. **Document Discovery** (`filesystem::docs`) - Markdown documents with content hashing
7. **Blast Radius** (`filesystem::blast_radius`) - Impact analysis for changed source files
8. **Justfile Detection** (`filesystem::just`) - Justfile discovery and recipe parsing
9. **Recent Commits** (`filesystem::git::recent_commits`) - Duration/hash/date-based commit queries

#### Git Detection

Uses the pure-Rust `gix` (gitoxide) crate for repository inspection. Every
production repository open rejects untrusted repositories (`bail_if_untrusted`),
and the existing out-of-process `git fetch --quiet --prune` is retained for
remote-tracking refresh.

**Key Types:**

- `GitInfo` - Complete git repository information
- `CommitInfo` - Commit SHA, author, message, remote sync status
- `RepoStatus` - Dirty files, staged/unstaged/untracked counts
- `RemoteInfo` - Remote name, URL, provider, branches (deep mode)
- `HostingProvider` - GitHub, GitLab, Bitbucket, etc.
- `BehindStatus` - Whether local branch is behind remote
- `WorktreeInfo` - Linked worktree information
- `WorktreeEntry` - Worktree name, branch, path, current flag, and detached-HEAD state
- `list_worktrees` - List all worktrees including the main worktree (sorted alphabetically)

**Detection Strategy:**

- **Standard Mode**: Local repository inspection (no network)
- **Deep Mode** (`--deep`): Queries remotes for branch lists and commit synchronization

**Example:**

```rust
use sniff::filesystem::git::detect_git;
use std::path::Path;

// Standard mode (no network)
let git = detect_git(Path::new("."), false, 10)?;
if let Some(info) = git {
    println!("Repository: {:?}", info.repo_root);
    println!("Branch: {:?}", info.current_branch);
    println!("Dirty: {}", info.status.as_ref().map_or(false, |s| s.is_dirty));
    println!("Commits ahead: {}", info.recent.len());

    for commit in info.recent.iter().take(5) {
        println!("  {} - {}", &commit.sha[..8], commit.message.lines().next().unwrap_or(""));
    }
}

// Deep mode (queries remotes)
let git_deep = detect_git(Path::new("."), true, 10)?;
if let Some(info) = git_deep {
    for remote in &info.remotes {
        println!("Remote {}: {:?}", remote.name, remote.provider);
        if let Some(ref branches) = remote.branches {
            println!("  Branches: {}", branches.len());
        }
    }

    // Check if behind
    if let Some(status) = &info.status {
        if let Some(behind) = &status.is_behind {
            match behind {
                sniff::filesystem::git::BehindStatus::NotBehind => {
                    println!("Up to date with remotes");
                }
                sniff::filesystem::git::BehindStatus::Behind(remotes) => {
                    println!("Behind: {}", remotes.join(", "));
                }
            }
        }
    }
}

// List all worktrees
if let Some(worktrees) = sniff::filesystem::git::list_worktrees(Path::new("."))? {
    for wt in &worktrees {
        let marker = if wt.is_current { "* " } else { "  " };
        let branch = wt.branch.as_deref().unwrap_or("detached HEAD");
        println!("{}{} (on {})", marker, wt.name, branch);
    }
}
```

#### Repository Detection

Detects monorepo standards, package structure, and acting binaries.

**Supported Standards:**

- Cargo workspaces (Rust)
- pnpm / npm / Yarn / Bun workspaces (JavaScript)
- uv workspaces (Python)
- Go workspaces
- Gradle multi-project builds and Maven multi-module builds (JVM)
- .NET solutions
- Bazel, Pants, Buck2 (polyglot build systems)
- Rush Stack
- Nx, Turborepo, Lerna (orchestrators layered on a membership authority)

**Key Types:**

- `RepoInfo` - Repository metadata and packages
- `MonorepoStandard` - Standard-based monorepo descriptor
- `MonorepoLayer` - One membership layer: authority + orchestrators + packages
- `DetectedStandard` - A matched standard with its resolved binary and confidence
- `Package` - Package path, languages, managers, dependencies
- `DependencyEntry` - Dependency with version requirements

**Example:**

```rust
use sniff::filesystem::repo::detect_repo;
use std::path::Path;

let repo = detect_repo(Path::new("."))?;
if let Some(info) = repo {
    if info.is_monorepo {
        // New topology model
        for layer in &info.monorepo_layers {
            println!("Authority: {:?}", layer.authority);
            println!("Orchestrators: {:?}", layer.orchestrators);
            println!("Packages: {}", layer.packages.len());
        }

        if let Some(packages) = info.packages {
            println!("Packages: {}", packages.len());
            for pkg in packages {
                println!("  {} at {}", pkg.name, pkg.path.display());
                println!("    Language: {:?}", pkg.primary_language);
                println!("    Managers: {:?}", pkg.package_managers);

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

**Topology JSON:**

When `RepoInfo` is serialized, the new keys appear only when populated:

- `monorepo_standards` — array of detected standards with resolved binary metadata.
- `monorepo_layers` — array of layers, each with `authority`, `orchestrators`, `provenance`, and `packages`.

#### Language Analysis

File extension-based language detection.

**Key Types:**

- `LanguageBreakdown` - Complete language statistics
- `LanguageStats` - Per-language file count and percentage

**Example:**

```rust
use sniff::filesystem::languages::detect_languages;
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
use sniff::package::{
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
use sniff::package::{enrich_dependencies, DependencyEntry, DependencyKind};

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

Detects installed programs across 9 categories with parallel execution, macOS app bundle support, and cwd-aware test-runner availability.

**Key Types:**

- `ProgramsInfo` - Aggregated detection results for all categories
- `ProgramMetadata` - Trait for program metadata (display name, description, website)
- `ExecutableSource` - How program was discovered (PATH, project-local bin, macOS bundle, or not found)
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
| Test Runners | cargo test, vitest, pytest, go test | project-local bins + PATH + parent binaries |

**Performance notes:**

`ProgramsInfo::detect()` builds a single shared `ExecutableIndex` by scanning every `PATH` directory and macOS app bundle location once, then runs all 9 categories in parallel. Per-program lookups are O(1) HashMap hits rather than repeated filesystem traversals, so the total cost is dominated by the one-time index build. Test runners also consult project-local bin directories and parent binaries, reporting whether each runner is installed, local, available via a parent, or not found.

**Example:**

```rust
use sniff::programs::ProgramsInfo;

// Detect all installed programs (parallel, shared index)
let programs = ProgramsInfo::detect();

println!("Editors: {:?}", programs.editors);
println!("Utilities: {:?}", programs.utilities);
println!("AI CLI tools: {:?}", programs.ai_clients);
println!("Test runners: {:?}", programs.test_runners);

// Access metadata
for editor in &programs.editors {
    println!("{}: {}", editor.display_name(), editor.description());
}
```

**macOS App Bundle Detection:**

```rust
use sniff::programs::find_program_with_source;

// Returns (Option<PathBuf>, ExecutableSource)
let (path, source) = find_program_with_source("code");
match source {
    ExecutableSource::Path => println!("Found in PATH"),
    ExecutableSource::MacOsBundle(bundle) => println!("Found in {}", bundle),
    ExecutableSource::NotFound => println!("Not installed"),
}
```

**Windows Fallback Chain:**

On Windows the executable search expands beyond PATH to cover registry-installed
GUI apps and traditional installers:

1. **PATH** — `CreateProcess`-compatible, returns `ExecutableSource::Path`.
2. **App Paths registry** — `HKCU` then `HKLM`
   (`SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`). HKCU wins ties to
   match `ShellExecuteEx`. Environment variables in the target path are
   expanded via `ExpandEnvironmentStringsW` and orphaned entries (whose target
   no longer exists) are dropped. Returns `ExecutableSource::WindowsAppPaths`.
3. **Install-root walk** — one directory deep under `%ProgramFiles%`,
   `%ProgramFiles(x86)%`, and `%LocalAppData%\Programs`. Returns
   `ExecutableSource::WindowsInstallRoot`. Catches VS Code-style
   user-scope installers that never register with App Paths.

The combined Windows scan costs 40–80 ms on a warm filesystem. It runs once
inside `ExecutableIndex::build()`, so the eight program-detection categories
amortize the cost.

### Services Module

Detects system services across multiple init systems.

**Key Types:**

- `ServicesInfo` - Init system detection result with service list
- `Service` - Individual service (name, running state, PID)
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
use sniff::services::{detect_services, ServiceState};

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

    #[error("Git error during {operation}: {source}")]
    Git {
        operation: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

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
network = ["dep:reqwest", "dep:tokio", "dep:futures", "reqwest/rustls"]
remote = ["network"]   # Enable remote repository inspection
```

**Network Feature:**

When enabled, provides:

- `CargoNetwork`, `NpmNetwork`, etc. implementations
- `enrich_dependencies()` async function
- Latest version resolution from package registries

**Remote Feature:**

When enabled, provides:

- `GitRemote::from_url()` for automatic provider detection
- Support for GitHub, GitLab, Gitea/Forgejo, and Bitbucket
- Remote repository metadata, pull requests, issues, tags, releases
- CI/CD detection for GitHub Actions, GitLab CI, Bitbucket Pipelines, etc.

### Remote Repository Inspection

The `remote` feature enables inspection of remote git repositories via hosting provider APIs.

**Automatic Provider Detection:**

```rust
use sniff::remote::{GitRemote, RemoteRepoProvider};

// Auto-detect provider from URL
let remote = GitRemote::from_url("https://github.com/rust-lang/cargo")?;
let parsed = GitRemote::parse_url("https://github.com/rust-lang/cargo")?;
let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;

println!("Stars: {:?}", report.metadata.stars);
println!("Open issues: {:?}", report.metadata.open_issues);
```

**Direct Provider Construction:**

```rust
use sniff::remote::{GitHubRemote, RemoteRepoProvider};

let provider = GitHubRemote::new()?;
let metadata = provider.get_repo_metadata("rust-lang", "cargo").await?;
```

**Supported Providers:**

| Provider | Public API | Self-Hosted | Auth Env Vars |
|----------|:----------:|:-----------:|---------------|
| GitHub | Yes | Yes (GHE) | `GITHUB_TOKEN`, `GH_TOKEN` |
| GitLab | Yes | Yes | `GITLAB_TOKEN`, `GITLAB_PRIVATE_TOKEN` |
| Gitea | - | Yes | `GITEA_TOKEN` |
| Bitbucket | Yes | Yes (DC) | `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` |

**URL Formats Supported:**

- HTTPS: `https://github.com/owner/repo`
- SSH: `git@github.com:owner/repo.git`
- GitLab nested groups: `https://gitlab.com/group/subgroup/repo`
- Self-hosted: `https://gitlab.example.com/team/project`

## Platform Support

| Platform | OS Detection | Hardware | Network | Git | GPU | Audio Devices |
|----------|:------------:|:--------:|:-------:|:---:|:---:|:-------------:|
| **Linux** | ✓ | ✓ | ✓ | ✓ | - | ✓ (PulseAudio/ALSA) |
| **macOS** | ✓ | ✓ | ✓ | ✓ | ✓ (Metal) | ✓ (Core Audio) |
| **Windows** | ✓ | ✓ | ✓ | ✓ | - | ✓ (PowerShell) |
| **BSD** | ✓ | ✓ | ✓ | ✓ | - | - |

**Notes:**

- GPU detection currently only supported on macOS (Metal API)
- Audio device detection uses platform-specific APIs (Core Audio, PulseAudio/ALSA, PowerShell)
- Network interface detection requires appropriate permissions
- Git operations require valid repository
- **WSL** is treated as the Linux compile and runtime path: under WSL, sniff
  uses the same `/proc`-backed detectors as native Linux, and `HostCapabilities`
  surfaces an `is_wsl` flag. No detector crosses into native Windows behavior
  when running inside WSL.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `sysinfo` | 0.38 | CPU, memory, storage detection |
| `gix` | =0.84.0 | Pure-Rust Git repository inspection (status, diff, history, refs, remotes, config, worktrees) |
| `biscuit-hash` | workspace | xxHash content hashing for document fingerprinting |
| `biscuit-file` | workspace | TOML/YAML file parsing |
| `getifaddrs` | 0.6 | Network interface enumeration |
| `hyperpolyglot` | 0.1 | Language detection |
| `rayon` | 1.11 | Parallel iteration for program detection |
| `which` | 8.0 | Executable discovery on PATH |
| `ec4rs` | 1 | EditorConfig resolution |
| `walkdir` | 2.5 | Recursive directory traversal |
| `reqwest` | 0.13 | HTTP client (network feature, optional) |
| `chrono` | 0.4 | Date/time handling |
| `strum` | 0.28 | Enum utilities and iteration |
| `thiserror` | 2.0 | Error type derivation |
| `serde` | 1.0 | Serialization support |
| `coreaudio-sys` | 0.2 | macOS audio device detection (target-specific) |
| `windows` | 0.62 | Windows service/audio detection (target-specific) |

## Testing

The library includes comprehensive unit tests for all modules:

```bash
# Run all tests
cargo test -p sniff

# Test specific modules
cargo test -p sniff os::
cargo test -p sniff hardware::
cargo test -p sniff network::
cargo test -p sniff filesystem::

# Test with network feature
cargo test -p sniff --features network
```

**Test Coverage:**

- OS detection: Distribution parsing, package manager detection
- Hardware: SIMD capabilities, serialization roundtrips
- Network: Interface detection, filtering, primary selection
- Filesystem: Git parsing, monorepo detection, language analysis
- Package: Manager detection, registry queries (network)

## Benchmarking and Profiling

The library ships with a Criterion benchmark harness under
`sniff/lib/benches/` and lightweight example binaries under
`sniff/lib/examples/` intended for flamegraph profiling.

```bash
# Run the full Criterion suite (HTML reports land in target/criterion)
just bench

# Run a single domain
just bench-system
just bench-hardware
just bench-filesystem
just bench-inventory

# End-to-end CLI benchmarks (requires `hyperfine` on PATH)
just bench-cli

# Generate a flamegraph from a profiling example
just profile profile_detect_full
just profile profile_filesystem
just profile profile_hardware

# Profile the sniff CLI directly
just profile-cli --json
```

Criterion output is written to `target/criterion/` with HTML reports at
`target/criterion/report/index.html`. Flamegraphs use a dedicated
`[profile.profiling]` Cargo profile (release + debuginfo, no stripping)
so sampled stacks resolve symbols without changing normal release
builds.

The `--perf` flag on the CLI exposes structured per-stage timings from
the library's `PerformanceCollector`; Criterion wall-clock numbers are
the primary regression surface, while `--perf` stage breakdowns are
useful for explaining *where* an observed regression lives.

**Platform notes:**

- Audio device and GPU/Metal benches are most meaningful on macOS.
  They still compile and run on Linux/Windows but mostly exercise
  stub paths.
- The `services_detect` bench depends on which init system is present
  (systemd, launchd, etc.) — variance between hosts is expected.

**Which benches track regressions:**

- `system::detect_summary` and `system::detect_full` are the headline
  regression signals.
- `filesystem_git::git_summary_monorepo`,
  `filesystem_repo::repo_structure_monorepo`, and
  `inventory::programs_detect` cover the highest-risk shared-walk and
  fan-out paths.
- `git_dirty_scaling::git_full/{N}` and
  `git_dirty_scaling::git_deep/{N}` measure how per-file diff cost
  scales with dirty file count (default `N` ∈ {10, 100}; set
  `SNIFF_BENCH_DEEP_DIRTY=1` to also run `N=1000`). Use these to
  validate the batched-diff aggregation work.
- `filesystem_docs::detect_docs_full` vs.
  `filesystem_docs::detect_blast_radius_only` quantify the win from
  the blast-radius-only frontmatter parser on a synthetic 200-doc
  fixture.
- `filesystem_repo::repo_full_monorepo` isolates the per-package
  language/framework refresh on a 90-package fixture.
- `programs::index_build_lazy` vs. `programs::index_build_eager_path`
  and `programs_bulk_lookup::lookup_lazy` vs.
  `programs_bulk_lookup::lookup_eager_path` measure the eager-PATH
  index trade-off used by `ProgramsInfo::detect`.
- Audio/GPU benches are informational and should not be used to gate
  merges across platforms.

### Phase 4 baseline measurements

Targeted profiling runs for ad-hoc baseline capture:

```bash
# Git dirty-file scaling (10/100/1000 dirty files; 1000 is opt-in)
SNIFF_BENCH_DEEP_DIRTY=1 cargo bench -p sniff --features network \
    --bench perf -- ^git_dirty_scaling

# Repo package-boundary refresh
cargo bench -p sniff --features network --bench perf -- ^filesystem_repo

# Docs full vs. blast-radius-only parser
cargo bench -p sniff --features network --bench perf -- ^filesystem_docs

# Eager vs. lazy PATH index, plus full ProgramsInfo fan-out
cargo bench -p sniff --features network --bench perf -- '^(programs|programs_bulk_lookup|programs_fanout)/'

# PATH-length scaling on the compiled CLI (requires hyperfine)
cargo build --release -p sniff-cli
SNIFF_BIN="$(cargo metadata --format-version 1 --no-deps \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/release/sniff"
SHORT_PATH="/usr/bin:/bin"
LONG_PATH="${PATH}"
hyperfine --warmup 3 \
    -n short_path "PATH=${SHORT_PATH} ${SNIFF_BIN} software --json >/dev/null" \
    -n full_path  "PATH=${LONG_PATH}  ${SNIFF_BIN} software --json >/dev/null"

# Flamegraph the high-fan-out CLI commands
cargo flamegraph --profile profiling -p sniff-cli -- repo git-status
cargo flamegraph --profile profiling -p sniff-cli -- software --json

# Compile-time / symbol-cost baselines for Phase 5.3
cargo bloat -p sniff-cli --release --bin sniff
cargo llvm-lines -p sniff-cli --release
```

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
use sniff::{filesystem::repo::detect_repo, package::enrich_dependencies};

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

Planned features:

- Expanded lockfile resolution for npm/pnpm/yarn and pip ecosystems
- Lockfile resolution for actual versions
- GPU detection for Windows (D3D12) and Linux (Vulkan)
- Runtime environment detection (Docker, VM, cloud providers)
- Performance profiling and benchmarking

## License

Part of the Rusty Biscuit monorepo. See top-level LICENSE file.
