# Sniff Library Design Document

A Rust library and CLI for detecting information about the host system across three domains: **Hardware**, **Network**, and **Filesystem/Repository**.

## Module Structure

```
sniff/
├── lib/
│   └── src/
│       ├── lib.rs              # Public API: detect(), SniffResult
│       ├── error.rs            # SniffError, Result alias
│       ├── hardware/
│       │   ├── mod.rs          # HardwareInfo, detect_hardware()
│       │   ├── cpu.rs          # CpuInfo
│       │   ├── memory.rs       # MemoryInfo
│       │   ├── gpu.rs          # GpuInfo (feature-gated)
│       │   └── storage.rs      # StorageInfo
│       ├── network/
│       │   ├── mod.rs          # NetworkInfo, detect_network()
│       │   └── interface.rs    # NetworkInterface, InterfaceFlags
│       └── filesystem/
│           ├── mod.rs          # FilesystemInfo, detect_filesystem()
│           ├── languages.rs    # LanguageBreakdown, PrimaryLanguage
│           ├── analysis.rs     # CodeAnalysis, analyze_code()
│           ├── git/
│           │   ├── mod.rs      # GitInfo
│           │   ├── remote.rs   # RemoteInfo, HostingProvider
│           │   ├── status.rs   # RepoStatus, FileStatus
│           │   └── branch.rs   # BranchInfo
│           ├── monorepo.rs     # MonorepoInfo, MonorepoTool
│           └── dependencies/
│               ├── mod.rs      # DependencyInfo
│               ├── package_manager.rs
│               ├── dependency.rs
│               ├── lockfile.rs
│               └── registry.rs # Registry API clients
└── cli/
    └── src/
        └── main.rs             # clap-based CLI
```

---

## Crate Dependencies

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `sysinfo` | 0.37+ | CPU, memory, disk, process info |
| `getifaddrs` | 0.6+ | Network interface enumeration |
| `git2` | 0.20+ | Git repository operations |
| `hyperpolyglot` | 0.1+ | Programming language detection |
| `serde` | 1.0 | Serialization for all output structs |
| `thiserror` | 2.0 | Error type definitions |
| `tokio` | 1.0 | Async runtime (optional, for registry queries) |

### Dependency Analysis

| Crate | Version | Purpose |
|-------|---------|---------|
| `semver` | 1.0 | Cargo-style version parsing |
| `nodejs-semver` | 4.0+ | npm/pnpm/yarn version range parsing |
| `package-parser` | 0.1+ | Multi-format lockfile parsing |
| `reqwest` | 0.12 | Registry API HTTP client |
| `url` | 2.5 | URL parsing for remotes |

### Optional/Feature-Gated

| Crate | Feature | Purpose |
|-------|---------|---------|
| `hardware-query` | `gpu` | Detailed GPU info (requires GPU drivers) |
| `walkdir` | default | Directory traversal |
| `rayon` | `parallel` | Parallel file scanning |

### Code Analysis

| Crate | Feature | Purpose |
|-------|---------|---------|
| `rust-code-analysis` | `analysis` | Code metrics (complexity, SLOC, maintainability) |

**Note:** `hyperpolyglot` is used for language *detection*, then `rust-code-analysis` provides deeper code *metrics* for detected languages. This two-phase approach leverages each crate's strengths.

---

## 1. Hardware Detection

### Data Structures

```rust
/// Aggregated hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub storage: Vec<StorageInfo>,
    pub gpus: Vec<GpuInfo>,  // feature = "gpu"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub name: String,           // "macOS", "Ubuntu", "Windows 11"
    pub version: String,        // "14.2.1", "22.04", "23H2"
    pub kernel: String,         // "Darwin 23.2.0", "6.5.0-14-generic"
    pub arch: String,           // "aarch64", "x86_64"
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub brand: String,          // "Apple M2 Pro"
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub frequency_mhz: Option<u64>,
    pub architecture: String,   // "arm64", "x86_64"
    /// Per-core usage (requires refresh, optional)
    pub usage_percent: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub name: String,           // "disk0s1", "nvme0n1p1"
    pub mount_point: PathBuf,
    pub fs_type: String,        // "apfs", "ext4", "ntfs"
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,           // "Apple M2 Pro", "NVIDIA RTX 4090"
    pub vendor: GpuVendor,
    pub vram_bytes: Option<u64>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuVendor {
    Apple,
    Nvidia,
    Amd,
    Intel,
    Unknown,
}
```

### API

```rust
/// Fast hardware detection (no CPU usage sampling)
pub fn detect_hardware() -> Result<HardwareInfo, SniffError>;

/// Hardware detection with CPU usage (requires ~1s sampling delay)
pub fn detect_hardware_with_usage() -> Result<HardwareInfo, SniffError>;
```

### Implementation Notes

- Use `sysinfo::System` with targeted `refresh_specifics()` for performance
- CPU usage requires two samples with delay; make this opt-in
- GPU detection via `hardware-query` is feature-gated due to driver dependencies
- `sysinfo` handles cross-platform differences; `hardware-query` provides deeper GPU info

---

## 2. Network Detection

### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub primary_interface: Option<String>,  // Best guess at default route
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,               // "en0", "eth0", "wlan0"
    pub index: u32,
    pub mac_address: Option<MacAddress>,
    pub ipv4_addresses: Vec<Ipv4Network>,
    pub ipv6_addresses: Vec<Ipv6Network>,
    pub flags: InterfaceFlags,
    pub mtu: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MacAddress(pub [u8; 6]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Network {
    pub address: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub broadcast: Option<Ipv4Addr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Network {
    pub address: Ipv6Addr,
    pub prefix_len: u8,
    pub scope: Ipv6Scope,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Ipv6Scope {
    Global,
    LinkLocal,
    SiteLocal,
    Loopback,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct InterfaceFlags {
    pub is_up: bool,
    pub is_loopback: bool,
    pub is_running: bool,
    pub supports_multicast: bool,
    pub supports_broadcast: bool,
}
```

### API

```rust
/// Enumerate all network interfaces
pub fn detect_network() -> Result<NetworkInfo, SniffError>;

/// Filter interfaces by criteria
pub fn detect_network_filtered(filter: InterfaceFilter) -> Result<NetworkInfo, SniffError>;

#[derive(Default)]
pub struct InterfaceFilter {
    pub include_loopback: bool,
    pub include_down: bool,
    pub ipv4_only: bool,
    pub ipv6_only: bool,
}
```

### Implementation Notes

- `getifaddrs` provides cross-platform interface enumeration
- Primary interface detection: prefer non-loopback interface with default gateway
- MAC address may be `None` on virtual interfaces or restricted systems

---

## 3. Filesystem/Repository Detection

### 3.1 Language Detection

Replace `rust-code-analysis` with `hyperpolyglot` for automatic detection.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageBreakdown {
    /// Languages sorted by file count (descending)
    pub languages: Vec<LanguageStats>,
    /// The dominant language (>50% of non-config files)
    pub primary: Option<PrimaryLanguage>,
    /// Total files analyzed
    pub total_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    pub language: String,           // "Rust", "TypeScript", "Python"
    pub file_count: usize,
    pub percentage: f32,            // 0.0 - 100.0
    pub detection_method: DetectionMethod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetectionMethod {
    Extension,      // Detected by file extension
    Filename,       // Detected by filename (e.g., "Makefile")
    Heuristics,     // Content-based detection
    Classifier,     // ML classifier fallback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryLanguage {
    pub language: String,
    pub confidence: LanguageConfidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LanguageConfidence {
    High,       // >80% of files
    Medium,     // 50-80% of files
    Low,        // Plurality but <50%
}
```

**Filtering Logic:**
- Exclude vendor directories (`vendor/`, `node_modules/`, `target/`, `.git/`)
- Exclude generated files (`.generated`, `.g.dart`, etc.)
- Weight by file count, not lines (faster, sufficient for detection)
- Shell scripts (Bash/Zsh) are noted but not treated as primary unless dominant

### 3.2 Code Analysis

**Crate:** `rust-code-analysis` (feature-gated: `analysis`)

After detecting languages with `hyperpolyglot`, use `rust-code-analysis` for code quality metrics:

1. **Phase 1** (hyperpolyglot): Detect what languages exist
2. **Phase 2** (rust-code-analysis): Compute metrics for supported languages

#### Supported Languages

Rust, C, C++, Python, JavaScript, TypeScript, Java, Go, Kotlin (via tree-sitter parsers)

#### Metrics

| Metric | Description |
|--------|-------------|
| SLOC/PLOC/CLOC | Source/Physical/Comment lines of code |
| Cyclomatic | Decision complexity (branches + 1) |
| Cognitive | Nesting-aware complexity |
| Halstead | Volume, difficulty, effort metrics |
| MI | Maintainability Index (0-100) |

#### Data Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysis {
    pub languages: Vec<LanguageMetrics>,
    pub summary: CodeSummary,
    pub hotspots: Option<Vec<CodeHotspot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetrics {
    pub language: String,
    pub file_count: usize,
    pub sloc: usize,
    pub avg_cyclomatic: f32,
    pub avg_cognitive: f32,
    pub max_cyclomatic: u32,
    pub maintainability_index: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeHotspot {
    pub path: PathBuf,
    pub function_name: Option<String>,
    pub line: usize,
    pub reason: HotspotReason,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotspotReason {
    HighCyclomatic,     // > 10
    HighCognitive,      // > 15
    LongFunction,       // > 100 SLOC
    LowMaintainability, // MI < 20
}
```

#### API

```rust
/// Analyze code metrics (opt-in, slower)
pub fn analyze_code(
    root: &Path,
    languages: &LanguageBreakdown,
    config: &AnalysisConfig,
) -> Result<CodeAnalysis, SniffError>;

#[derive(Debug, Clone, Default)]
pub struct AnalysisConfig {
    pub detect_hotspots: bool,
    pub max_files: Option<usize>,
    pub skip_tests: bool,
}
```

#### Performance

| Operation | ~Time (1k files) |
|-----------|-----------------|
| File metrics | 2-5s |
| With hotspots | 5-10s |
| Full + Halstead | 10-20s |

**Optimization:** Use `rayon` for parallel analysis; cache by path + mtime.

### 3.3 Git Information

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: PathBuf,
    pub current_branch: Option<String>,
    pub head_commit: Option<CommitInfo>,
    pub status: RepoStatus,
    pub remotes: Vec<RemoteInfo>,
    pub branches: BranchSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,                // Full 40-char SHA
    pub short_sha: String,          // 7-char abbreviated
    pub message: String,            // First line only
    pub author: String,
    pub author_email: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub is_dirty: bool,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub conflicts_count: usize,
    /// Detailed file statuses (optional, can be expensive)
    pub files: Option<Vec<FileStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: PathBuf,
    pub status: FileStatusKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileStatusKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,               // "origin", "upstream"
    pub url: String,
    pub push_url: Option<String>,   // If different from fetch URL
    pub hosting_provider: HostingProvider,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HostingProvider {
    GitHub,
    GitLab,
    Bitbucket,
    AzureDevOps,
    AwsCodeCommit,
    Gitea,
    Forgejo,
    SourceHut,
    SelfHosted,     // Detected as git but unknown provider
    Unknown,        // Could not parse URL
}

impl HostingProvider {
    /// Parse hosting provider from remote URL
    pub fn from_url(url: &str) -> Self {
        // Handle both HTTPS and SSH URLs
        let normalized = url
            .trim_start_matches("git@")
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("ssh://");

        if normalized.starts_with("github.com") {
            Self::GitHub
        } else if normalized.starts_with("gitlab.com") {
            Self::GitLab
        } else if normalized.starts_with("bitbucket.org") {
            Self::Bitbucket
        } else if normalized.contains("dev.azure.com") || normalized.contains("visualstudio.com") {
            Self::AzureDevOps
        } else if normalized.contains("codecommit") && normalized.contains("amazonaws.com") {
            Self::AwsCodeCommit
        } else if normalized.contains("sr.ht") {
            Self::SourceHut
        // Gitea/Forgejo detected by API probe or config, not URL alone
        } else if normalized.contains("git") {
            Self::SelfHosted
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummary {
    pub local: Vec<BranchInfo>,
    pub remote: Vec<BranchInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,       // "origin/main"
    pub last_commit: CommitInfo,
    /// Ahead/behind tracking branch (if upstream set)
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}
```

### 3.4 Monorepo Detection

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonorepoInfo {
    pub is_monorepo: bool,
    pub tool: Option<MonorepoTool>,
    pub root: PathBuf,
    pub packages: Vec<PackageLocation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonorepoTool {
    // JavaScript/TypeScript
    NpmWorkspaces,
    PnpmWorkspaces,
    YarnWorkspaces,
    BunWorkspaces,
    Nx,
    Lerna,
    Turborepo,
    Rush,

    // Multi-language
    Bazel,
    Pants,
    Buck2,
    Moon,

    // Rust
    CargoWorkspace,

    // Java/JVM
    GradleMultiProject,
    MavenMultiModule,

    // Python
    PoetrySelf,     // Poetry with path dependencies
    Hatch,

    // Other
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLocation {
    pub name: String,
    pub path: PathBuf,              // Relative to repo root
    pub package_manager: Option<PackageManager>,
}
```

**Detection Strategy:**

| Tool | Detection File | Key Field |
|------|----------------|-----------|
| pnpm | `pnpm-workspace.yaml` | `packages:` |
| npm | `package.json` | `workspaces:` |
| yarn | `package.json` | `workspaces:` |
| Nx | `nx.json` | existence |
| Turborepo | `turbo.json` | existence |
| Lerna | `lerna.json` | existence |
| Cargo | `Cargo.toml` | `[workspace]` |
| Bazel | `WORKSPACE` or `MODULE.bazel` | existence |
| Pants | `pants.toml` | existence |
| Gradle | `settings.gradle(.kts)` | `include` |
| Maven | `pom.xml` | `<modules>` |

---

## 4. Dependencies (Production Design)

### 4.1 Core Types

```rust
/// A package manager ecosystem
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PackageManager {
    // JavaScript/TypeScript
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
    Jsr,

    // Rust
    Cargo,

    // Python
    Pip,
    Poetry,
    Pdm,
    Uv,
    Conda,
    Pipenv,

    // Ruby
    Bundler,

    // PHP
    Composer,

    // Java/JVM
    Maven,
    Gradle,

    // Go
    GoMod,

    // .NET
    NuGet,
    Paket,

    // C/C++
    Vcpkg,
    Conan,

    // Lua
    LuaRocks,

    // Swift
    SwiftPM,

    // Elixir
    Hex,

    // Other
    Unknown,
}

impl PackageManager {
    /// The primary language this package manager serves
    pub fn primary_language(&self) -> &'static str {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn | Self::Bun | Self::Deno | Self::Jsr => "JavaScript",
            Self::Cargo => "Rust",
            Self::Pip | Self::Poetry | Self::Pdm | Self::Uv | Self::Conda | Self::Pipenv => "Python",
            Self::Bundler => "Ruby",
            Self::Composer => "PHP",
            Self::Maven | Self::Gradle => "Java",
            Self::GoMod => "Go",
            Self::NuGet | Self::Paket => "C#",
            Self::Vcpkg | Self::Conan => "C++",
            Self::LuaRocks => "Lua",
            Self::SwiftPM => "Swift",
            Self::Hex => "Elixir",
            Self::Unknown => "Unknown",
        }
    }

    /// The canonical package registry URL
    pub fn registry_url(&self) -> Option<&'static str> {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn | Self::Bun => Some("https://registry.npmjs.org"),
            Self::Jsr => Some("https://jsr.io"),
            Self::Cargo => Some("https://crates.io"),
            Self::Pip | Self::Poetry | Self::Pdm | Self::Uv | Self::Pipenv => Some("https://pypi.org"),
            Self::Bundler => Some("https://rubygems.org"),
            Self::Composer => Some("https://packagist.org"),
            Self::Maven => Some("https://repo1.maven.org/maven2"),
            Self::GoMod => Some("https://pkg.go.dev"),
            Self::NuGet => Some("https://api.nuget.org/v3"),
            Self::LuaRocks => Some("https://luarocks.org"),
            Self::Hex => Some("https://hex.pm"),
            _ => None,
        }
    }

    /// Parent package manager (for npm-compatible managers)
    pub fn parent(&self) -> Option<Self> {
        match self {
            Self::Pnpm | Self::Yarn | Self::Bun => Some(Self::Npm),
            Self::Uv | Self::Pdm | Self::Poetry | Self::Pipenv => Some(Self::Pip),
            _ => None,
        }
    }

    /// Manifest filename(s) for this package manager
    pub fn manifest_files(&self) -> &'static [&'static str] {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn | Self::Bun => &["package.json"],
            Self::Cargo => &["Cargo.toml"],
            Self::Pip => &["requirements.txt", "setup.py", "pyproject.toml"],
            Self::Poetry | Self::Pdm | Self::Uv => &["pyproject.toml"],
            Self::Pipenv => &["Pipfile"],
            Self::Bundler => &["Gemfile"],
            Self::Composer => &["composer.json"],
            Self::Maven => &["pom.xml"],
            Self::Gradle => &["build.gradle", "build.gradle.kts"],
            Self::GoMod => &["go.mod"],
            Self::NuGet => &["*.csproj", "*.fsproj", "packages.config"],
            Self::SwiftPM => &["Package.swift"],
            _ => &[],
        }
    }

    /// Lockfile filename(s) for this package manager
    pub fn lockfiles(&self) -> &'static [&'static str] {
        match self {
            Self::Npm => &["package-lock.json", "npm-shrinkwrap.json"],
            Self::Pnpm => &["pnpm-lock.yaml"],
            Self::Yarn => &["yarn.lock"],
            Self::Bun => &["bun.lockb"],
            Self::Cargo => &["Cargo.lock"],
            Self::Poetry => &["poetry.lock"],
            Self::Pdm => &["pdm.lock"],
            Self::Uv => &["uv.lock"],
            Self::Pipenv => &["Pipfile.lock"],
            Self::Bundler => &["Gemfile.lock"],
            Self::Composer => &["composer.lock"],
            Self::GoMod => &["go.sum"],
            _ => &[],
        }
    }
}
```

### 4.2 Dependency Representation

```rust
/// Relationship type between a project and its dependency
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencyKind {
    /// Runtime dependency (production)
    Runtime,
    /// Development-only dependency
    Dev,
    /// Peer dependency (npm ecosystem)
    Peer,
    /// Optional dependency
    Optional,
    /// Build-time dependency
    Build,
}

/// A single dependency declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name on the registry
    pub name: String,

    /// The dependency relationship
    pub kind: DependencyKind,

    /// Declared version requirement (e.g., "^1.2.3", ">=1.0,<2.0")
    pub version_requirement: String,

    /// Resolved version from lockfile (if available)
    pub resolved_version: Option<String>,

    /// Package manager this dependency belongs to
    pub package_manager: PackageManager,

    /// Additional metadata
    pub metadata: DependencyMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyMetadata {
    /// Registry this package comes from (if not default)
    pub registry: Option<String>,

    /// Git repository source (for git dependencies)
    pub git_url: Option<String>,
    pub git_ref: Option<String>,

    /// Local path (for path dependencies)
    pub path: Option<PathBuf>,

    /// Workspace dependency marker
    pub is_workspace: bool,

    /// Features enabled (Cargo)
    pub features: Vec<String>,

    /// Whether this is an alias (npm "alias": "npm:real-package@version")
    pub alias_for: Option<String>,
}
```

### 4.3 Version Information (Registry Query Results)

```rust
/// Extended dependency info from registry queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyVersionInfo {
    /// Base dependency declaration
    pub dependency: Dependency,

    /// Latest version available on registry
    pub latest_version: Option<String>,

    /// Latest version satisfying the version requirement
    pub latest_satisfying: Option<String>,

    /// Whether an update is available within semver constraints
    pub update_available: UpdateStatus,

    /// Security advisory information
    pub advisories: Vec<SecurityAdvisory>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Already at latest compatible version
    UpToDate,
    /// Patch update available (1.2.3 -> 1.2.4)
    PatchAvailable,
    /// Minor update available (1.2.3 -> 1.3.0)
    MinorAvailable,
    /// Major update available (1.2.3 -> 2.0.0)
    MajorAvailable,
    /// Could not determine (no lockfile or registry error)
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAdvisory {
    pub id: String,             // "GHSA-xxxx-yyyy-zzzz", "CVE-2024-1234"
    pub severity: AdvisorySeverity,
    pub title: String,
    pub url: Option<String>,
    pub patched_versions: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdvisorySeverity {
    Low,
    Moderate,
    High,
    Critical,
}
```

### 4.4 Aggregated Dependency Report

```rust
/// Complete dependency analysis for a project/monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyReport {
    /// Package manager(s) detected
    pub package_managers: Vec<PackageManager>,

    /// Detected dependencies by package (for monorepos)
    pub packages: Vec<PackageDependencies>,

    /// Summary statistics
    pub summary: DependencySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependencies {
    /// Package name (from manifest)
    pub name: Option<String>,

    /// Package path relative to repo root
    pub path: PathBuf,

    /// Package manager for this package
    pub package_manager: PackageManager,

    /// Has lockfile
    pub has_lockfile: bool,

    /// All dependencies
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencySummary {
    pub total_dependencies: usize,
    pub runtime_dependencies: usize,
    pub dev_dependencies: usize,
    pub outdated_count: usize,
    pub security_issues: usize,

    /// Unique dependencies across all packages
    pub unique_dependencies: usize,
}
```

### 4.5 Version Parsing Strategy

```rust
/// Unified version requirement parser
pub struct VersionRequirement {
    raw: String,
    kind: VersionRequirementKind,
}

enum VersionRequirementKind {
    /// Cargo-style: "1.2.3", "^1.2", ">=1,<2"
    Cargo(semver::VersionReq),
    /// npm-style: "^1.2.3", "~1.2.3", "1.x", ">=1.0.0 <2.0.0"
    Npm(nodejs_semver::Range),
    /// Exact version or URL/path
    Other(String),
}

impl VersionRequirement {
    pub fn parse(raw: &str, pm: PackageManager) -> Self {
        let kind = match pm {
            PackageManager::Cargo => {
                semver::VersionReq::parse(raw)
                    .map(VersionRequirementKind::Cargo)
                    .unwrap_or_else(|_| VersionRequirementKind::Other(raw.to_string()))
            }
            PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn | PackageManager::Bun => {
                nodejs_semver::Range::parse(raw)
                    .map(VersionRequirementKind::Npm)
                    .unwrap_or_else(|_| VersionRequirementKind::Other(raw.to_string()))
            }
            _ => VersionRequirementKind::Other(raw.to_string()),
        };
        Self { raw: raw.to_string(), kind }
    }

    pub fn matches(&self, version: &str) -> bool {
        match &self.kind {
            VersionRequirementKind::Cargo(req) => {
                semver::Version::parse(version)
                    .map(|v| req.matches(&v))
                    .unwrap_or(false)
            }
            VersionRequirementKind::Npm(range) => {
                nodejs_semver::Version::parse(version)
                    .map(|v| range.satisfies(&v))
                    .unwrap_or(false)
            }
            VersionRequirementKind::Other(_) => false,
        }
    }
}
```

### 4.6 Registry Client Architecture

```rust
/// Trait for querying package registries
#[async_trait]
pub trait RegistryClient: Send + Sync {
    /// Get metadata for a package
    async fn get_package(&self, name: &str) -> Result<PackageMetadata, RegistryError>;

    /// Get all versions for a package
    async fn get_versions(&self, name: &str) -> Result<Vec<String>, RegistryError>;

    /// Check for security advisories
    async fn get_advisories(&self, name: &str, version: &str) -> Result<Vec<SecurityAdvisory>, RegistryError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub description: Option<String>,
    pub latest_version: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
}

/// Registry implementations
pub struct NpmRegistry { client: reqwest::Client }
pub struct CratesIoRegistry { client: reqwest::Client }
pub struct PyPiRegistry { client: reqwest::Client }
// ... etc

/// Factory for creating registry clients
pub fn registry_for(pm: PackageManager) -> Option<Box<dyn RegistryClient>> {
    match pm {
        PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn | PackageManager::Bun => {
            Some(Box::new(NpmRegistry::new()))
        }
        PackageManager::Cargo => Some(Box::new(CratesIoRegistry::new())),
        PackageManager::Pip | PackageManager::Poetry | PackageManager::Pdm | PackageManager::Uv => {
            Some(Box::new(PyPiRegistry::new()))
        }
        _ => None,
    }
}
```

### 4.7 Lockfile Parsing

Use `package-parser` crate for multi-format support, with custom handling for unsupported formats.

```rust
/// Parse dependencies from a project directory
pub fn parse_dependencies(root: &Path) -> Result<Vec<PackageDependencies>, SniffError> {
    let mut packages = Vec::new();

    // Detect package managers present
    let managers = detect_package_managers(root)?;

    for (pm, manifest_path) in managers {
        let deps = match pm {
            PackageManager::Cargo => parse_cargo_toml(&manifest_path)?,
            PackageManager::Npm | PackageManager::Pnpm | PackageManager::Yarn | PackageManager::Bun => {
                parse_package_json(&manifest_path)?
            }
            PackageManager::Poetry | PackageManager::Pdm | PackageManager::Uv => {
                parse_pyproject_toml(&manifest_path)?
            }
            // ... other parsers
            _ => continue,
        };

        // Try to resolve versions from lockfile
        let lockfile_path = find_lockfile(&manifest_path, pm);
        let resolved = if let Some(lf) = lockfile_path {
            resolve_from_lockfile(&lf, &deps, pm)?
        } else {
            deps
        };

        packages.push(PackageDependencies {
            name: extract_package_name(&manifest_path, pm),
            path: manifest_path.parent().unwrap().to_path_buf(),
            package_manager: pm,
            has_lockfile: lockfile_path.is_some(),
            dependencies: resolved,
        });
    }

    Ok(packages)
}

/// Detect which package managers are in use
fn detect_package_managers(root: &Path) -> Result<Vec<(PackageManager, PathBuf)>, SniffError> {
    let mut found = Vec::new();

    // Priority order for detection
    let checks = [
        ("Cargo.toml", PackageManager::Cargo),
        ("package.json", PackageManager::Npm),  // Refined by lockfile below
        ("pyproject.toml", PackageManager::Poetry),  // Refined by content
        ("Gemfile", PackageManager::Bundler),
        ("composer.json", PackageManager::Composer),
        ("go.mod", PackageManager::GoMod),
    ];

    for (filename, pm) in checks {
        if let Some(path) = find_file_upward(root, filename) {
            let refined_pm = refine_package_manager(&path, pm);
            found.push((refined_pm, path));
        }
    }

    Ok(found)
}

/// Refine npm -> pnpm/yarn/bun based on lockfile
fn refine_package_manager(manifest: &Path, initial: PackageManager) -> PackageManager {
    if initial != PackageManager::Npm {
        return initial;
    }

    let dir = manifest.parent().unwrap();
    if dir.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if dir.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else if dir.join("bun.lockb").exists() {
        PackageManager::Bun
    } else {
        PackageManager::Npm
    }
}
```

---

## 5. Aggregated API

### Main Entry Point

```rust
/// Complete sniff result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    pub hardware: HardwareInfo,
    pub network: NetworkInfo,
    pub filesystem: Option<FilesystemInfo>,  // None if no directory specified
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub base_dir: PathBuf,
    pub languages: LanguageBreakdown,
    pub code_analysis: Option<CodeAnalysis>,  // Opt-in via config
    pub git: Option<GitInfo>,
    pub monorepo: Option<MonorepoInfo>,
    pub dependencies: DependencyReport,
}

/// Configuration for detection
#[derive(Debug, Clone, Default)]
pub struct SniffConfig {
    /// Base directory for filesystem analysis (default: CWD)
    pub base_dir: Option<PathBuf>,

    /// Include CPU usage sampling (adds ~1s delay)
    pub include_cpu_usage: bool,

    /// Include detailed file status in git info
    pub include_file_status: bool,

    /// Run code analysis with rust-code-analysis (adds 2-10s)
    pub include_code_analysis: bool,

    /// Code analysis configuration (if include_code_analysis is true)
    pub analysis_config: Option<AnalysisConfig>,

    /// Query registries for version info (requires network)
    pub query_registries: bool,

    /// Skip certain detectors
    pub skip_hardware: bool,
    pub skip_network: bool,
    pub skip_filesystem: bool,
}

/// Fast detection with minimal I/O
pub fn detect() -> Result<SniffResult, SniffError> {
    detect_with_config(SniffConfig::default())
}

/// Detection with custom configuration
pub fn detect_with_config(config: SniffConfig) -> Result<SniffResult, SniffError> {
    let hardware = if config.skip_hardware {
        HardwareInfo::default()
    } else if config.include_cpu_usage {
        detect_hardware_with_usage()?
    } else {
        detect_hardware()?
    };

    let network = if config.skip_network {
        NetworkInfo::default()
    } else {
        detect_network()?
    };

    let filesystem = if config.skip_filesystem {
        None
    } else {
        let base = config.base_dir.unwrap_or_else(|| std::env::current_dir().unwrap());
        Some(detect_filesystem(&base, &config)?)
    };

    Ok(SniffResult { hardware, network, filesystem })
}

/// Async detection with registry queries
pub async fn detect_async(config: SniffConfig) -> Result<SniffResult, SniffError> {
    // Same as sync but allows registry queries
    todo!()
}
```

---

## 6. CLI Design

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sniff", about = "Detect system and repository information")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Base directory for filesystem analysis
    #[arg(short, long, global = true)]
    base: Option<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Detect all information (default)
    All {
        /// Include CPU usage (slower)
        #[arg(long)]
        cpu_usage: bool,

        /// Query registries for latest versions
        #[arg(long)]
        check_updates: bool,
    },

    /// Hardware information only
    Hardware {
        #[arg(long)]
        cpu_usage: bool,
    },

    /// Network information only
    Network {
        /// Include loopback interfaces
        #[arg(long)]
        include_loopback: bool,
    },

    /// Filesystem/repo information only
    Filesystem,

    /// Git repository status
    Git {
        /// Show detailed file status
        #[arg(long)]
        files: bool,
    },

    /// Programming language breakdown
    Languages,

    /// Dependency analysis
    Dependencies {
        /// Check for outdated dependencies
        #[arg(long)]
        outdated: bool,

        /// Check for security advisories
        #[arg(long)]
        audit: bool,
    },

    /// Monorepo structure
    Monorepo,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Yaml,
}
```

---

## 7. Performance Considerations

### Fast Path (Default)

| Operation | Expected Time |
|-----------|---------------|
| Hardware (no CPU usage) | <10ms |
| Network interfaces | <5ms |
| Language detection (1000 files) | <100ms |
| Git status | <50ms |
| Dependency parsing | <100ms |
| **Total** | **<300ms** |

### Slow Path (Opt-in)

| Operation | Expected Time |
|-----------|---------------|
| CPU usage sampling | ~1000ms |
| Code analysis (1k files) | 2-5s |
| Code analysis + hotspots | 5-10s |
| Registry queries (10 deps, parallel) | 500-2000ms |
| Large monorepo (10k files) | 500-1000ms |

### Optimization Strategies

1. **Parallel scanning**: Use `rayon` for directory traversal
2. **Early termination**: Stop language detection after 10k files
3. **Caching**: Cache registry responses for 24 hours
4. **Incremental**: Track git status with libgit2's efficient diff
5. **Lazy fields**: `Option<T>` for expensive computations

---

## 8. Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum SniffError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    #[error("Manifest parse error in {path}: {message}")]
    ManifestParse { path: PathBuf, message: String },

    #[error("Lockfile parse error in {path}: {message}")]
    LockfileParse { path: PathBuf, message: String },

    #[error("Registry error for {registry}: {message}")]
    Registry { registry: String, message: String },

    #[error("Unsupported platform for {feature}")]
    UnsupportedPlatform { feature: String },
}

pub type Result<T> = std::result::Result<T, SniffError>;
```

---

## 9. Testing Strategy

### Unit Tests

- Version parsing for each package manager
- Host provider URL detection
- Monorepo tool detection from config files
- Dependency kind mapping

### Integration Tests

- Real repository scanning (use fixtures or temp repos)
- Lockfile parsing with real-world examples
- Network interface enumeration (mock on CI)

### Fixtures

```
tests/fixtures/
├── repos/
│   ├── rust-simple/          # Single Cargo.toml
│   ├── npm-workspace/        # npm workspaces
│   ├── pnpm-monorepo/        # pnpm with catalog
│   ├── mixed-monorepo/       # Nx with multiple languages
│   └── python-poetry/        # Poetry project
├── lockfiles/
│   ├── package-lock.json
│   ├── pnpm-lock.yaml
│   ├── Cargo.lock
│   ├── poetry.lock
│   └── yarn.lock
└── manifests/
    ├── package.json
    ├── Cargo.toml
    └── pyproject.toml
```

---

## 10. Future Extensions

1. **Security scanning**: Integrate with vulnerability databases (OSV, npm audit, cargo-audit)
2. **License detection**: Parse license fields and files
3. **CI detection**: Detect CI system from environment and config files
4. **Docker/Container detection**: Parse Dockerfile, docker-compose.yml
5. **Cloud provider detection**: AWS/GCP/Azure from environment or config
6. **Build system detection**: Make, CMake, Meson, Gradle, etc.
